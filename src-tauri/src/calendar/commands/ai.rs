use tauri::{AppHandle, State};

use crate::calendar::ai::{ollama::OllamaClient, prompts};
use crate::calendar::db::{queries, DbPool};
use crate::calendar::models::error::CalError;
use crate::calendar::models::event::ViewMode;
use crate::calendar::models::summary::AiDigest;

const DIGEST_MAX_AGE_MS: i64 = 24 * 60 * 60 * 1000;

#[tauri::command]
pub async fn generate_day_briefing(
    date: String,
    db: State<'_, DbPool>,
    ai: State<'_, OllamaClient>,
) -> Result<String, CalError> {
    // Check cache
    if let Some(cached) = queries::get_digest(&db, "day", &date)? {
        let age = chrono::Utc::now().timestamp_millis() - cached.created_at;
        if age < DIGEST_MAX_AGE_MS {
            return Ok(cached.content);
        }
    }

    let calendar_data = queries::query_calendar_data(&db, ViewMode::Day, &date)?;
    let notes = queries::get_day_notes(&db, &date)?;
    let notes_text = notes.content.unwrap_or_default();

    let prompt = prompts::day_briefing_prompt(&date, &calendar_data.events, &notes_text);
    let content = ai.generate(&prompt).await?;

    queries::upsert_digest(&db, "day", &date, &content, Some(&ai.model()?))?;

    Ok(content)
}

#[tauri::command]
pub async fn generate_week_summary(
    week_key: String,
    anchor_date: String,
    db: State<'_, DbPool>,
    ai: State<'_, OllamaClient>,
) -> Result<String, CalError> {
    // Check cache
    if let Some(cached) = queries::get_digest(&db, "week", &week_key)? {
        let age = chrono::Utc::now().timestamp_millis() - cached.created_at;
        if age < DIGEST_MAX_AGE_MS {
            return Ok(cached.content);
        }
    }

    let calendar_data = queries::query_calendar_data(&db, ViewMode::Week, &anchor_date)?;
    let notes_text = calendar_data
        .notes
        .iter()
        .filter_map(|n| n.content.as_deref())
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = prompts::week_digest_prompt(&week_key, &calendar_data.events, &notes_text);
    let content = ai.generate(&prompt).await?;

    queries::upsert_digest(&db, "week", &week_key, &content, Some(&ai.model()?))?;

    Ok(content)
}

#[tauri::command]
pub async fn stream_ai_response(
    prompt: String,
    app: AppHandle,
    ai: State<'_, OllamaClient>,
) -> Result<(), CalError> {
    ai.stream_to_window(&prompt, &app).await
}

#[tauri::command]
pub async fn invalidate_day_digest(
    date: String,
    db: State<'_, DbPool>,
) -> Result<(), CalError> {
    queries::delete_day_digest(&db, &date)
}

#[tauri::command]
pub async fn get_cached_digest(
    period_type: String,
    period_key: String,
    db: State<'_, DbPool>,
) -> Result<Option<AiDigest>, CalError> {
    queries::get_digest(&db, &period_type, &period_key)
}
