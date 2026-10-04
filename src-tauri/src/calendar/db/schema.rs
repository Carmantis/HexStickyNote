/// Ordered list of SQL migrations applied at startup.
/// Only append — never modify existing entries.
pub const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"CREATE TABLE IF NOT EXISTS events (
        id           TEXT PRIMARY KEY,
        title        TEXT NOT NULL,
        description  TEXT,
        start_ts     INTEGER NOT NULL,
        end_ts       INTEGER NOT NULL,
        all_day      INTEGER NOT NULL DEFAULT 0,
        location     TEXT,
        color        TEXT NOT NULL DEFAULT '#4A90D9',
        recurrence   TEXT,
        source       TEXT NOT NULL DEFAULT 'local'
    )"#,
    r#"CREATE TABLE IF NOT EXISTS reminders (
        id           TEXT PRIMARY KEY,
        event_id     TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
        offset_min   INTEGER NOT NULL,
        notified     INTEGER NOT NULL DEFAULT 0
    )"#,
    r#"CREATE TABLE IF NOT EXISTS notes (
        id           TEXT PRIMARY KEY,
        date         TEXT NOT NULL UNIQUE,
        content      TEXT,
        ai_summary   TEXT,
        updated_at   INTEGER
    )"#,
    r#"CREATE TABLE IF NOT EXISTS ai_digests (
        id           TEXT PRIMARY KEY,
        period_type  TEXT NOT NULL,
        period_key   TEXT NOT NULL,
        content      TEXT NOT NULL,
        model        TEXT,
        created_at   INTEGER NOT NULL
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_events_start ON events(start_ts)",
    "CREATE INDEX IF NOT EXISTS idx_notes_date ON notes(date)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_digests_period ON ai_digests(period_type, period_key)",
];

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn all_migrations_apply_cleanly() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        for migration in MIGRATIONS {
            conn.execute_batch(migration).unwrap();
        }
    }
}
