use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time;

use crate::calendar::db::{queries, DbPool};

const TICK_INTERVAL_SECS: u64 = 60;

/// Spawns a background Tokio task that fires notifications for due reminders.
pub fn start(db: DbPool, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(TICK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            check_reminders(&db, &app).await;
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
