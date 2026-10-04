use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::calendar::db::DbPool;
use crate::calendar::models::error::CalError;
use crate::calendar::models::event::{
    CalendarData, CreateEventDto, CreateReminderDto, DayNotes, Event, Reminder, UpdateEventDto,
    ViewMode,
};
use crate::calendar::models::summary::AiDigest;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

pub fn insert_event(pool: &DbPool, dto: CreateEventDto) -> Result<Event, CalError> {
    let conn = pool.lock()?;
    let id = Uuid::new_v4().to_string();
    let color = dto.color.unwrap_or_else(|| "#4A90D9".to_string());
    let all_day = dto.all_day.unwrap_or(false) as i64;

    conn.execute(
        "INSERT INTO events (id, title, description, start_ts, end_ts, all_day, location, color, recurrence, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'local')",
        params![
            id,
            dto.title,
            dto.description,
            dto.start_ts,
            dto.end_ts,
            all_day,
            dto.location,
            color,
            dto.recurrence,
        ],
    )?;

    query_event_by_id(&conn, &id)
}

pub fn update_event(pool: &DbPool, dto: UpdateEventDto) -> Result<Event, CalError> {
    let conn = pool.lock()?;

    // Fetch current state first
    let current = query_event_by_id(&conn, &dto.id)?;

    let title = dto.title.unwrap_or(current.title);
    let description = dto.description.or(current.description);
    let start_ts = dto.start_ts.unwrap_or(current.start_ts);
    let end_ts = dto.end_ts.unwrap_or(current.end_ts);
    let all_day = dto.all_day.unwrap_or(current.all_day) as i64;
    let location = dto.location.or(current.location);
    let color = dto.color.unwrap_or(current.color);
    let recurrence = dto.recurrence.or(current.recurrence);

    conn.execute(
        "UPDATE events SET title=?1, description=?2, start_ts=?3, end_ts=?4,
         all_day=?5, location=?6, color=?7, recurrence=?8 WHERE id=?9",
        params![title, description, start_ts, end_ts, all_day, location, color, recurrence, dto.id],
    )?;

    query_event_by_id(&conn, &dto.id)
}

pub fn delete_event(pool: &DbPool, id: &str) -> Result<(), CalError> {
    let conn = pool.lock()?;
    let affected = conn.execute("DELETE FROM events WHERE id=?1", params![id])?;
    if affected == 0 {
        return Err(CalError::NotFound(format!("Event '{}' not found", id)));
    }
    Ok(())
}

pub fn query_events_in_range(pool: &DbPool, start_ts: i64, end_ts: i64) -> Result<Vec<Event>, CalError> {
    let conn = pool.lock()?;
    let mut stmt = conn.prepare(
        "SELECT id, title, description, start_ts, end_ts, all_day, location, color, recurrence, source
         FROM events WHERE start_ts < ?1 AND end_ts > ?2 ORDER BY start_ts ASC",
    )?;

    let events = stmt
        .query_map(params![end_ts, start_ts], row_to_event)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(events)
}

fn query_event_by_id(conn: &rusqlite::Connection, id: &str) -> Result<Event, CalError> {
    conn.query_row(
        "SELECT id, title, description, start_ts, end_ts, all_day, location, color, recurrence, source
         FROM events WHERE id=?1",
        params![id],
        row_to_event,
    )
    .map_err(|_| CalError::NotFound(format!("Event '{}' not found", id)))
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        start_ts: row.get(3)?,
        end_ts: row.get(4)?,
        all_day: row.get::<_, i64>(5)? != 0,
        location: row.get(6)?,
        color: row.get(7)?,
        recurrence: row.get(8)?,
        source: row.get(9)?,
    })
}

// ---------------------------------------------------------------------------
// Calendar data (aggregated fetch for a view)
// ---------------------------------------------------------------------------

pub fn query_calendar_data(
    pool: &DbPool,
    mode: ViewMode,
    anchor_date: &str,
) -> Result<CalendarData, CalError> {
    let (start_ts, end_ts) = date_range_for_mode(&mode, anchor_date)?;
    let events = query_events_in_range(pool, start_ts, end_ts)?;
    let notes = query_notes_in_range(pool, anchor_date, &mode)?;

    Ok(CalendarData {
        mode,
        anchor_date: anchor_date.to_string(),
        events,
        notes,
    })
}

fn date_range_for_mode(mode: &ViewMode, anchor: &str) -> Result<(i64, i64), CalError> {
    use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};

    let date = NaiveDate::parse_from_str(anchor, "%Y-%m-%d")
        .map_err(|e| CalError::Validation(e.to_string()))?;

    let (start, end) = match mode {
        ViewMode::Day => {
            let s = date;
            let e = s + Duration::days(1);
            (s, e)
        }
        ViewMode::Week => {
            // ISO week: Monday to Sunday
            let dow = date.weekday().num_days_from_monday() as i64;
            let monday = date - Duration::days(dow);
            let sunday = monday + Duration::days(7);
            (monday, sunday)
        }
        ViewMode::Month => {
            let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
                .ok_or_else(|| CalError::Validation("Invalid date".to_string()))?;
            // Include a full 6-row grid: up to 6 weeks
            let dow = first.weekday().num_days_from_monday() as i64;
            let grid_start = first - Duration::days(dow);
            let grid_end = grid_start + Duration::days(42);
            (grid_start, grid_end)
        }
    };

    let start_ts = Utc.from_utc_datetime(&start.and_hms_opt(0, 0, 0).unwrap()).timestamp_millis();
    let end_ts = Utc.from_utc_datetime(&end.and_hms_opt(0, 0, 0).unwrap()).timestamp_millis();
    Ok((start_ts, end_ts))
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

pub fn query_notes_in_range(
    pool: &DbPool,
    anchor: &str,
    mode: &ViewMode,
) -> Result<Vec<DayNotes>, CalError> {
    use chrono::{Datelike, Duration, NaiveDate};

    let date = NaiveDate::parse_from_str(anchor, "%Y-%m-%d")
        .map_err(|e| CalError::Validation(e.to_string()))?;

    let (start_date, end_date) = match mode {
        ViewMode::Day => (date, date + Duration::days(1)),
        ViewMode::Week => {
            let dow = date.weekday().num_days_from_monday() as i64;
            let monday = date - Duration::days(dow);
            (monday, monday + Duration::days(7))
        }
        ViewMode::Month => {
            let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
                .ok_or_else(|| CalError::Validation("Invalid date".to_string()))?;
            let dow = first.weekday().num_days_from_monday() as i64;
            let grid_start = first - Duration::days(dow);
            (grid_start, grid_start + Duration::days(42))
        }
    };

    let conn = pool.lock()?;
    let mut stmt = conn.prepare(
        "SELECT id, date, content, ai_summary, updated_at FROM notes
         WHERE date >= ?1 AND date < ?2 ORDER BY date ASC",
    )?;

    let notes = stmt
        .query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| {
                Ok(DayNotes {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    content: row.get(2)?,
                    ai_summary: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(notes)
}

pub fn upsert_day_notes(pool: &DbPool, date: &str, content: &str) -> Result<DayNotes, CalError> {
    let conn = pool.lock()?;

    let existing_id: Option<String> = conn
        .query_row("SELECT id FROM notes WHERE date=?1", params![date], |r| r.get(0))
        .optional()?;

    let id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO notes (id, date, content, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(date) DO UPDATE SET content=excluded.content, updated_at=excluded.updated_at",
        params![id, date, content, now],
    )?;

    Ok(conn.query_row(
        "SELECT id, date, content, ai_summary, updated_at FROM notes WHERE date=?1",
        params![date],
        |row| {
            Ok(DayNotes {
                id: row.get(0)?,
                date: row.get(1)?,
                content: row.get(2)?,
                ai_summary: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )?)
}

pub fn get_day_notes(pool: &DbPool, date: &str) -> Result<DayNotes, CalError> {
    let conn = pool.lock()?;

    let notes = conn
        .query_row(
            "SELECT id, date, content, ai_summary, updated_at FROM notes WHERE date=?1",
            params![date],
            |row| {
                Ok(DayNotes {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    content: row.get(2)?,
                    ai_summary: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()?
        .unwrap_or_else(|| DayNotes {
            id: None,
            date: date.to_string(),
            content: None,
            ai_summary: None,
            updated_at: None,
        });

    Ok(notes)
}

// ---------------------------------------------------------------------------
// Reminders
// ---------------------------------------------------------------------------

pub fn insert_reminder(pool: &DbPool, dto: CreateReminderDto) -> Result<Reminder, CalError> {
    let conn = pool.lock()?;
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO reminders (id, event_id, offset_min, notified) VALUES (?1, ?2, ?3, 0)",
        params![id, dto.event_id, dto.offset_min],
    )?;

    Ok(Reminder {
        id,
        event_id: dto.event_id,
        offset_min: dto.offset_min,
        notified: false,
    })
}

pub fn get_pending_reminders(pool: &DbPool) -> Result<Vec<(Reminder, Event)>, CalError> {
    let conn = pool.lock()?;
    let now_ms = chrono::Utc::now().timestamp_millis();

    let mut stmt = conn.prepare(
        "SELECT r.id, r.event_id, r.offset_min, r.notified,
                e.id, e.title, e.description, e.start_ts, e.end_ts, e.all_day,
                e.location, e.color, e.recurrence, e.source
         FROM reminders r
         JOIN events e ON e.id = r.event_id
         WHERE r.notified = 0
           AND (e.start_ts - (r.offset_min * 60000)) <= ?1",
    )?;

    let results = stmt
        .query_map(params![now_ms], |row| {
            let reminder = Reminder {
                id: row.get(0)?,
                event_id: row.get(1)?,
                offset_min: row.get(2)?,
                notified: row.get::<_, i64>(3)? != 0,
            };
            let event = Event {
                id: row.get(4)?,
                title: row.get(5)?,
                description: row.get(6)?,
                start_ts: row.get(7)?,
                end_ts: row.get(8)?,
                all_day: row.get::<_, i64>(9)? != 0,
                location: row.get(10)?,
                color: row.get(11)?,
                recurrence: row.get(12)?,
                source: row.get(13)?,
            };
            Ok((reminder, event))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn mark_reminder_notified(pool: &DbPool, reminder_id: &str) -> Result<(), CalError> {
    let conn = pool.lock()?;
    conn.execute(
        "UPDATE reminders SET notified=1 WHERE id=?1",
        params![reminder_id],
    )?;
    Ok(())
}

pub fn dismiss_reminder(pool: &DbPool, reminder_id: &str) -> Result<(), CalError> {
    let conn = pool.lock()?;
    let affected = conn.execute("DELETE FROM reminders WHERE id=?1", params![reminder_id])?;
    if affected == 0 {
        return Err(CalError::NotFound(format!("Reminder '{}' not found", reminder_id)));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AI digests
// ---------------------------------------------------------------------------

pub fn get_digest(pool: &DbPool, period_type: &str, period_key: &str) -> Result<Option<AiDigest>, CalError> {
    let conn = pool.lock()?;
    let digest = conn
        .query_row(
            "SELECT id, period_type, period_key, content, model, created_at FROM ai_digests
             WHERE period_type=?1 AND period_key=?2",
            params![period_type, period_key],
            |row| {
                Ok(AiDigest {
                    id: row.get(0)?,
                    period_type: row.get(1)?,
                    period_key: row.get(2)?,
                    content: row.get(3)?,
                    model: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .optional()?;

    Ok(digest)
}

pub fn delete_day_digest(pool: &DbPool, date: &str) -> Result<(), CalError> {
    let conn = pool.lock()?;
    conn.execute(
        "DELETE FROM ai_digests WHERE period_type='day' AND period_key=?1",
        params![date],
    )?;
    Ok(())
}

pub fn upsert_digest(
    pool: &DbPool,
    period_type: &str,
    period_key: &str,
    content: &str,
    model: Option<&str>,
) -> Result<AiDigest, CalError> {
    let conn = pool.lock()?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO ai_digests (id, period_type, period_key, content, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(period_type, period_key) DO UPDATE SET
             content=excluded.content, model=excluded.model, created_at=excluded.created_at",
        params![id, period_type, period_key, content, model, now],
    )?;

    Ok(AiDigest {
        id,
        period_type: period_type.to_string(),
        period_key: period_key.to_string(),
        content: content.to_string(),
        model: model.map(String::from),
        created_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::db::DbPool;

    fn test_pool() -> DbPool {
        DbPool::open_in_memory().unwrap()
    }

    #[test]
    fn create_and_delete_event() {
        let pool = test_pool();
        let dto = CreateEventDto {
            title: "Test event".to_string(),
            description: None,
            start_ts: 1_700_000_000_000,
            end_ts: 1_700_003_600_000,
            all_day: Some(false),
            location: None,
            color: None,
            recurrence: None,
        };
        let event = insert_event(&pool, dto).unwrap();
        assert_eq!(event.title, "Test event");

        delete_event(&pool, &event.id).unwrap();
        let result = delete_event(&pool, &event.id);
        assert!(matches!(result, Err(CalError::NotFound(_))));
    }

    #[test]
    fn upsert_notes() {
        let pool = test_pool();
        let notes = upsert_day_notes(&pool, "2025-03-25", "Hello").unwrap();
        assert_eq!(notes.content, Some("Hello".to_string()));

        let updated = upsert_day_notes(&pool, "2025-03-25", "Updated").unwrap();
        assert_eq!(updated.content, Some("Updated".to_string()));
        assert_eq!(notes.id, updated.id);
    }
}
