use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time;

use crate::calendar::ai::ollama::OllamaClient;
use crate::calendar::ai::prompts;
use crate::calendar::db::{queries, DbPool};
use crate::calendar::models::event::ViewMode;

const TICK_INTERVAL_SECS: u64 = 60;
const DIGEST_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000; // 24 hours

/// Spawns a background Tokio task that periodically:
///   1. Fires OS notifications for due reminders.
///   2. Generates missing weekly digests.
pub fn start(db: DbPool, ollama: OllamaClient, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            check_reminders(&db, &app).await;
            check_weekly_digest(&db, &ollama).await;
        }
    });
}

async fn check_reminders(db: &DbPool, app: &AppHandle) {
    let pairs = match queries::get_pending_reminders(db) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[scheduler] Failed to fetch reminders: {}", e);
            return;
        }
    };

    for (reminder, event) in pairs {
        let title = format!("Reminder: {}", event.title);
        let body = format!(
            "Starts in {} min",
            reminder.offset_min
        );

        // Emit to frontend — the frontend handles the OS notification via
        // tauri-plugin-notification because it requires a JS-side permission grant.
        if let Err(e) = app.emit(
            "reminder_due",
            serde_json::json!({
                "event_id": event.id,
                "event_title": event.title,
                "start_ts": event.start_ts,
                "title": title,
                "body": body,
            }),
        ) {
            eprintln!("[scheduler] Failed to emit reminder_due: {}", e);
        }

        if let Err(e) = queries::mark_reminder_notified(db, &reminder.id) {
            eprintln!("[scheduler] Failed to mark reminder notified: {}", e);
        }
    }
}

async fn check_weekly_digest(db: &DbPool, ollama: &OllamaClient) {
    // No Ollama model selected: nothing to generate with
    let Ok(model) = ollama.model() else {
        return;
    };

    let today = chrono::Utc::now();
    let week_key = today.format("%G-W%V").to_string(); // ISO week, e.g. "2025-W12"
    let anchor = today.format("%Y-%m-%d").to_string();

    // Check whether a fresh digest already exists
    let existing = queries::get_digest(db, "week", &week_key);
    match existing {
        Ok(Some(digest)) => {
            let age_ms = chrono::Utc::now().timestamp_millis() - digest.created_at;
            if age_ms < DIGEST_MAX_AGE_MS {
                return; // Still fresh
            }
        }
        Ok(None) => {} // No digest — generate one
        Err(e) => {
            eprintln!("[scheduler] Digest check failed: {}", e);
            return;
        }
    }

    // Fetch this week's events and notes
    let calendar_data = match queries::query_calendar_data(db, ViewMode::Week, &anchor) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[scheduler] Failed to fetch week data: {}", e);
            return;
        }
    };

    let notes_text = calendar_data
        .notes
        .iter()
        .filter_map(|n| n.content.as_deref())
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = prompts::week_digest_prompt(&week_key, &calendar_data.events, &notes_text);

    match ollama.generate(&prompt).await {
        Ok(content) => {
            if let Err(e) =
                queries::upsert_digest(db, "week", &week_key, &content, Some(&model))
            {
                eprintln!("[scheduler] Failed to store digest: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[scheduler] Digest generation failed (Ollama unreachable?): {}", e);
        }
    }
}
