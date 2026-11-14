use tauri::State;

use crate::calendar::db::{queries, DbPool};
use crate::calendar::models::error::CalError;
use crate::calendar::models::event::{CreateReminderDto, Reminder};

#[tauri::command]
pub async fn schedule_reminder(
    payload: CreateReminderDto,
    db: State<'_, DbPool>,
) -> Result<Reminder, CalError> {
    queries::insert_reminder(&db, payload)
}

#[tauri::command]
pub async fn dismiss_reminder(
    reminder_id: String,
    db: State<'_, DbPool>,
) -> Result<(), CalError> {
    queries::dismiss_reminder(&db, &reminder_id)
}
