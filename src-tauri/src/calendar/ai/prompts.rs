use crate::calendar::models::event::Event;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_events(events: &[Event]) -> String {
    if events.is_empty() {
        return "No events scheduled.".to_string();
    }

    events
        .iter()
        .map(|e| {
            let time = if e.all_day {
                "All day".to_string()
            } else {
                let start = chrono::DateTime::from_timestamp_millis(e.start_ts)
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "?".to_string());
                let end = chrono::DateTime::from_timestamp_millis(e.end_ts)
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "?".to_string());
                format!("{} – {}", start, end)
            };
            let mut line = format!("- {} ({})", e.title, time);
            if let Some(loc) = &e.location {
                if !loc.trim().is_empty() {
                    line.push_str(&format!(", location: {}", loc));
                }
            }
            if let Some(desc) = &e.description {
                if !desc.trim().is_empty() {
                    line.push_str(&format!(", description: {}", desc));
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Daily briefing
// ---------------------------------------------------------------------------

pub fn day_briefing_prompt(date: &str, events: &[Event], notes: &str) -> String {
    format!(
        r#"You are HexCalendar's personal assistant.
Today is {date}. Analyse the day's events and notes.
Provide a concise summary (max 3 sentences).
Highlight the most important tasks and any scheduling conflicts.

Events:
{events}

Notes:
{notes}

Reply in English only, without markdown formatting."#,
        date = date,
        events = format_events(events),
        notes = if notes.trim().is_empty() { "No notes." } else { notes },
    )
}

// ---------------------------------------------------------------------------
// Weekly digest
// ---------------------------------------------------------------------------

pub fn week_digest_prompt(week_key: &str, events: &[Event], notes: &str) -> String {
    format!(
        r#"You are HexCalendar's personal assistant.
Generate a weekly digest for {week_key}.

Produce a markdown document with exactly three sections:
## Week highlights
## Next week priorities
## Notes & observations

Base your summary on the following data.

Events this week:
{events}

Notes this week:
{notes}

Keep each section to 3–5 bullet points. Write in English only."#,
        week_key = week_key,
        events = format_events(events),
        notes = if notes.trim().is_empty() { "No notes." } else { notes },
    )
}

// ---------------------------------------------------------------------------
// Smart reminder
// ---------------------------------------------------------------------------

pub fn conflict_detection_prompt(events: &[Event]) -> String {
    format!(
        r#"You are HexCalendar's scheduling assistant.
Review the following events and identify any scheduling conflicts,
back-to-back meetings without breaks, or overloaded periods.
Be concise — one sentence per issue found. If no issues, say "No conflicts detected."

Events:
{events}"#,
        events = format_events(events),
    )
}
