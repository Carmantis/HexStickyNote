//! HexCalendar, merged into HexStickyNote
//!
//! Events, day notes, reminders and AI digests stored in SQLite. The code comes
//! from the standalone HexCalendar app; the database stays in that app's data
//! directory so both share the same calendar.

pub mod ai;
pub mod commands;
pub mod db;
pub mod models;

use crate::settings_manager::SettingsManager;
use ai::{ollama::OllamaClient, scheduler};
use db::DbPool;
use std::sync::Arc;
use tauri::Manager;

/// Open the calendar database, register calendar state and start the reminder scheduler
pub fn init(app: &tauri::App, settings: Arc<SettingsManager>) -> Result<(), models::error::CalError> {
    let db = DbPool::init()?;
    app.manage(db.clone());

    let ollama = OllamaClient::new(settings);
    app.manage(ollama.clone());

    scheduler::start(db, ollama, app.handle().clone());
    Ok(())
}
