//! HexCalendar, merged into HexStickyNote
//!
//! Events, day notes and reminders stored in SQLite. The code comes from the
//! standalone HexCalendar app; the database stays in that app's data directory
//! so both share the same calendar. AI features live in the app-wide assistant
//! (see crate::assistant), not here.

pub mod commands;
pub mod db;
pub mod models;
pub mod reminders;

use db::DbPool;
use tauri::Manager;

/// Open the calendar database, register calendar state and start the reminder scheduler
pub fn init(app: &tauri::App) -> Result<(), models::error::CalError> {
    let db = DbPool::init()?;
    app.manage(db.clone());

    reminders::start(db, app.handle().clone());
    Ok(())
}
