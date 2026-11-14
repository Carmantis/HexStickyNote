pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::calendar::models::error::CalError;
use schema::MIGRATIONS;

#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
}

impl DbPool {
    /// Opens the database where the standalone HexCalendar keeps it
    /// (Tauri's app_data_dir for the `com.hexcalendar.app` identifier).
    pub fn init() -> Result<Self, CalError> {
        let app_dir = directories::BaseDirs::new()
            .ok_or_else(|| CalError::Io("Failed to determine data directory".to_string()))?
            .data_dir()
            .join("com.hexcalendar.app");

        std::fs::create_dir_all(&app_dir)?;

        let db_path = app_dir.join("hexcalendar.db");
        Self::open_at(db_path)
    }

    /// Opens (or creates) the database at a specific path.
    pub fn open_at(path: impl AsRef<std::path::Path>) -> Result<Self, CalError> {
        let conn = Connection::open(path)?;
        Self::setup(conn)
    }

    /// Opens an in-memory database — used in tests.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, CalError> {
        let conn = Connection::open_in_memory()?;
        Self::setup(conn)
    }

    fn setup(conn: Connection) -> Result<Self, CalError> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        for migration in MIGRATIONS {
            conn.execute_batch(migration)?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, CalError> {
        self.conn
            .lock()
            .map_err(|e| CalError::Database(format!("Mutex poisoned: {}", e)))
    }
}
