use tauri::State;

use crate::calendar::db::{queries, DbPool};
use crate::calendar::models::error::CalError;
use crate::calendar::models::event::{CalendarData, CreateEventDto, DayNotes, Event, UpdateEventDto, ViewMode};

#[tauri::command]
pub async fn get_calendar_data(
    mode: ViewMode,
    anchor_date: String,
    db: State<'_, DbPool>,
) -> Result<CalendarData, CalError> {
    queries::query_calendar_data(&db, mode, &anchor_date)
}

#[tauri::command]
pub async fn create_event(
    payload: CreateEventDto,
    db: State<'_, DbPool>,
) -> Result<Event, CalError> {
    queries::insert_event(&db, payload)
}

#[tauri::command]
pub async fn update_event(
    payload: UpdateEventDto,
    db: State<'_, DbPool>,
) -> Result<Event, CalError> {
    queries::update_event(&db, payload)
}

#[tauri::command]
pub async fn delete_event(id: String, db: State<'_, DbPool>) -> Result<(), CalError> {
    queries::delete_event(&db, &id)
}

#[tauri::command]
pub async fn get_day_notes(date: String, db: State<'_, DbPool>) -> Result<DayNotes, CalError> {
    queries::get_day_notes(&db, &date)
}

#[tauri::command]
pub async fn save_day_notes(
    date: String,
    content: String,
    db: State<'_, DbPool>,
) -> Result<DayNotes, CalError> {
    queries::upsert_day_notes(&db, &date, &content)
}
