//! Assistant tools
//!
//! What the assistant can do across the three apps: read and create notes,
//! calendar events and HexTime entries. Read tools run straight away; write
//! tools change data and only run after the user confirms them. There are no
//! delete tools.
//!
//! Dates are local calendar dates (`YYYY-MM-DD`) and times local wall-clock
//! times (`HH:MM`), which is what people say and what models produce reliably.

use crate::calendar::db::{queries, DbPool};
use crate::calendar::models::event::{CreateEventDto, Event};
use crate::card_manager;
use crate::hextime::HexTime;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("{0} is not available")]
    Unavailable(String),
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Only reads data; runs without asking
    Read,
    /// Changes data; runs only after the user confirms
    Write,
}

/// What the tools work on
pub struct ToolContext<'a> {
    /// None when the calendar failed to initialise
    pub calendar: Option<&'a DbPool>,
    pub hextime: &'a HexTime,
    pub client: &'a Client,
}

struct ToolSpec {
    name: &'static str,
    kind: ToolKind,
    description: &'static str,
    parameters: fn() -> Value,
}

fn no_parameters() -> Value {
    json!({ "type": "object", "properties": {} })
}

fn date_range_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "from_date": { "type": "string", "description": "First day, YYYY-MM-DD" },
            "to_date": { "type": "string", "description": "Last day (inclusive), YYYY-MM-DD. Defaults to from_date." }
        },
        "required": ["from_date"]
    })
}

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "list_events",
        kind: ToolKind::Read,
        description: "List calendar events between two dates. Recurring events appear only at their first occurrence.",
        parameters: date_range_parameters,
    },
    ToolSpec {
        name: "create_event",
        kind: ToolKind::Write,
        description: "Create a calendar event. Without start_time it is an all-day event; without end_time it lasts one hour.",
        parameters: || {
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "date": { "type": "string", "description": "YYYY-MM-DD" },
                    "start_time": { "type": "string", "description": "HH:MM, 24-hour clock" },
                    "end_time": { "type": "string", "description": "HH:MM, 24-hour clock" },
                    "location": { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["title", "date"]
            })
        },
    },
    ToolSpec {
        name: "list_notes",
        kind: ToolKind::Read,
        description: "List all sticky notes with their id, title and last update date.",
        parameters: no_parameters,
    },
    ToolSpec {
        name: "read_note",
        kind: ToolKind::Read,
        description: "Read the full Markdown content of a sticky note.",
        parameters: || {
            json!({
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Note id from list_notes" } },
                "required": ["id"]
            })
        },
    },
    ToolSpec {
        name: "create_note",
        kind: ToolKind::Write,
        description: "Create a new sticky note. Content is Markdown; start it with a '# Title' line.",
        parameters: || {
            json!({
                "type": "object",
                "properties": { "content": { "type": "string" } },
                "required": ["content"]
            })
        },
    },
    ToolSpec {
        name: "list_time_entries",
        kind: ToolKind::Read,
        description: "List tracked time entries (HexTime) between two dates, with project, description and duration.",
        parameters: date_range_parameters,
    },
    ToolSpec {
        name: "get_timer",
        kind: ToolKind::Read,
        description: "Show the running time tracking timer, if any.",
        parameters: no_parameters,
    },
    ToolSpec {
        name: "start_timer",
        kind: ToolKind::Write,
        description: "Start the time tracking timer. Stops a timer that is already running.",
        parameters: || {
            json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string", "description": "What is being worked on" },
                    "project": { "type": "string", "description": "Name of an existing HexTime project" }
                }
            })
        },
    },
    ToolSpec {
        name: "stop_timer",
        kind: ToolKind::Write,
        description: "Stop the running time tracking timer.",
        parameters: no_parameters,
    },
];

fn spec(name: &str) -> Result<&'static ToolSpec, ToolError> {
    TOOLS
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| ToolError::UnknownTool(name.to_string()))
}

/// Tool definitions in Ollama's chat API format
pub fn definitions() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": (t.parameters)()
                }
            })
        })
        .collect()
}

pub fn kind(name: &str) -> Result<ToolKind, ToolError> {
    Ok(spec(name)?.kind)
}

// ============================================================================
// Arguments
// ============================================================================

fn args<T: for<'de> Deserialize<'de>>(value: &Value) -> Result<T, ToolError> {
    // Models sometimes send no arguments at all for parameterless tools
    let value = if value.is_null() { json!({}) } else { value.clone() };
    serde_json::from_value(value).map_err(|e| ToolError::InvalidArguments(e.to_string()))
}

#[derive(Deserialize)]
struct DateRangeArgs {
    from_date: String,
    to_date: Option<String>,
}

#[derive(Deserialize)]
struct CreateEventArgs {
    title: String,
    date: String,
    start_time: Option<String>,
    end_time: Option<String>,
    location: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct ReadNoteArgs {
    id: String,
}

#[derive(Deserialize)]
struct CreateNoteArgs {
    content: String,
}

#[derive(Deserialize)]
struct StartTimerArgs {
    description: Option<String>,
    project: Option<String>,
}

fn parse_date(s: &str) -> Result<NaiveDate, ToolError> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| ToolError::InvalidArguments(format!("'{}' is not a YYYY-MM-DD date", s)))
}

fn parse_time(s: &str) -> Result<NaiveTime, ToolError> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| ToolError::InvalidArguments(format!("'{}' is not an HH:MM time", s)))
}

fn local(date: NaiveDate, time: NaiveTime) -> Result<DateTime<Local>, ToolError> {
    Local
        .from_local_datetime(&date.and_time(time))
        .earliest()
        .ok_or_else(|| ToolError::InvalidArguments(format!("{} {} does not exist locally", date, time)))
}

/// Start of `from` and start of the day after `to`, both local
fn day_range(range: &DateRangeArgs) -> Result<(DateTime<Local>, DateTime<Local>), ToolError> {
    let from = parse_date(&range.from_date)?;
    let to = match &range.to_date {
        Some(to) if !to.trim().is_empty() => parse_date(to)?,
        _ => from,
    };
    if to < from {
        return Err(ToolError::InvalidArguments("to_date is before from_date".to_string()));
    }
    let next = to.succ_opt().ok_or_else(|| ToolError::InvalidArguments("date out of range".to_string()))?;
    Ok((local(from, NaiveTime::MIN)?, local(next, NaiveTime::MIN)?))
}

/// The event an argument set describes, as stored by the calendar
fn event_dto(a: &CreateEventArgs) -> Result<CreateEventDto, ToolError> {
    let title = a.title.trim();
    if title.is_empty() {
        return Err(ToolError::InvalidArguments("title is empty".to_string()));
    }
    let date = parse_date(&a.date)?;

    let (start, end, all_day) = match a.start_time.as_deref().filter(|t| !t.trim().is_empty()) {
        None => {
            let next = date.succ_opt().ok_or_else(|| ToolError::InvalidArguments("date out of range".to_string()))?;
            (local(date, NaiveTime::MIN)?, local(next, NaiveTime::MIN)?, true)
        }
        Some(start_time) => {
            let start = local(date, parse_time(start_time)?)?;
            let end = match a.end_time.as_deref().filter(|t| !t.trim().is_empty()) {
                Some(end_time) => local(date, parse_time(end_time)?)?,
                None => start + chrono::Duration::hours(1),
            };
            if end <= start {
                return Err(ToolError::InvalidArguments("end_time must be after start_time".to_string()));
            }
            (start, end, false)
        }
    };

    let non_empty = |s: &Option<String>| s.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    Ok(CreateEventDto {
        title: title.to_string(),
        description: non_empty(&a.description),
        start_ts: start.timestamp_millis(),
        end_ts: end.timestamp_millis(),
        all_day: Some(all_day),
        location: non_empty(&a.location),
        color: None,
        recurrence: None,
    })
}

// ============================================================================
// Confirmation text
// ============================================================================

fn local_from_millis(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

/// A one-line description of what a write tool will do, for the confirmation card
pub fn describe(name: &str, arguments: &Value) -> Result<String, ToolError> {
    match name {
        "create_event" => {
            let dto = event_dto(&args::<CreateEventArgs>(arguments)?)?;
            let start = local_from_millis(dto.start_ts).ok_or_else(|| ToolError::Failed("bad time".into()))?;
            let end = local_from_millis(dto.end_ts).ok_or_else(|| ToolError::Failed("bad time".into()))?;
            let when = if dto.all_day == Some(true) {
                format!("{} (all day)", start.format("%a %-d %b %Y"))
            } else {
                format!("{}, {}–{}", start.format("%a %-d %b %Y"), start.format("%H:%M"), end.format("%H:%M"))
            };
            let place = dto.location.map(|l| format!(" at {}", l)).unwrap_or_default();
            Ok(format!("Create event “{}” on {}{}", dto.title, when, place))
        }
        "create_note" => {
            let a = args::<CreateNoteArgs>(arguments)?;
            Ok(format!("Create note “{}”", card_manager::extract_title_from_content(&a.content)))
        }
        "start_timer" => {
            let a = args::<StartTimerArgs>(arguments)?;
            let what = a.description.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| "no description".to_string());
            let project = a.project.filter(|p| !p.trim().is_empty()).map(|p| format!(" on {}", p)).unwrap_or_default();
            Ok(format!("Start timer: {}{}", what, project))
        }
        "stop_timer" => Ok("Stop the running timer".to_string()),
        other => {
            spec(other)?;
            Ok(format!("Run {}", other))
        }
    }
}

// ============================================================================
// Execution
// ============================================================================

fn event_json(e: &Event) -> Value {
    let start = local_from_millis(e.start_ts);
    let end = local_from_millis(e.end_ts);
    json!({
        "title": e.title,
        "date": start.map(|d| d.format("%Y-%m-%d (%A)").to_string()),
        "start": if e.all_day { None } else { start.map(|d| d.format("%H:%M").to_string()) },
        "end": if e.all_day { None } else { end.map(|d| d.format("%H:%M").to_string()) },
        "all_day": e.all_day,
        "location": e.location,
        "description": e.description,
        "recurring": e.recurrence.is_some(),
    })
}

fn calendar<'a>(ctx: &ToolContext<'a>) -> Result<&'a DbPool, ToolError> {
    ctx.calendar.ok_or_else(|| ToolError::Unavailable("The calendar".to_string()))
}

/// Call the HexTime API (starting the sidecar if needed) and return its JSON
async fn hextime(
    ctx: &ToolContext<'_>,
    method: reqwest::Method,
    path: &str,
    query: &[(&str, String)],
    body: Option<Value>,
) -> Result<Value, ToolError> {
    let base = ctx
        .hextime
        .start(ctx.client)
        .await
        .map_err(|e| ToolError::Unavailable(format!("Time tracking ({})", e)))?;

    let mut request = ctx.client.request(method, format!("{}/api/v1{}", base, path)).query(query);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|e| ToolError::Failed(e.to_string()))?;
    let status = response.status();
    let value: Value = response.json().await.unwrap_or(Value::Null);

    if !status.is_success() {
        // HexTime errors look like {"error": {"code": ..., "message": ...}}
        let message = value["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("HexTime returned {}", status));
        return Err(ToolError::Failed(message));
    }
    Ok(value)
}

/// Project id -> name, for showing entries by project name
async fn projects(ctx: &ToolContext<'_>) -> Result<Vec<(String, String)>, ToolError> {
    let list = hextime(ctx, reqwest::Method::GET, "/projects", &[], None).await?;
    Ok(list
        .as_array()
        .map(|projects| {
            projects
                .iter()
                .filter_map(|p| Some((p["id"].as_str()?.to_string(), p["name"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default())
}

fn entry_json(entry: &Value, projects: &[(String, String)]) -> Value {
    let local_time = |field: &str| {
        entry[field]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.with_timezone(&Local))
    };
    let project = entry["project_id"]
        .as_str()
        .and_then(|id| projects.iter().find(|(pid, _)| pid == id))
        .map(|(_, name)| name.clone());

    json!({
        "description": entry["description"],
        "project": project,
        "date": local_time("started_at").map(|t| t.format("%Y-%m-%d (%A)").to_string()),
        "start": local_time("started_at").map(|t| t.format("%H:%M").to_string()),
        "end": local_time("ended_at").map(|t| t.format("%H:%M").to_string()),
        "running": entry["ended_at"].is_null(),
        "duration_minutes": entry["duration_seconds"].as_i64().map(|s| s / 60),
        "billable": entry["billable"],
    })
}

/// Run a tool and return its result as JSON for the model
pub async fn execute(ctx: &ToolContext<'_>, name: &str, arguments: &Value) -> Result<Value, ToolError> {
    match name {
        "list_events" => {
            let (from, to) = day_range(&args(arguments)?)?;
            let events = queries::query_events_in_range(calendar(ctx)?, from.timestamp_millis(), to.timestamp_millis())
                .map_err(|e| ToolError::Failed(e.to_string()))?;
            Ok(json!({ "events": events.iter().map(event_json).collect::<Vec<_>>() }))
        }
        "create_event" => {
            let dto = event_dto(&args(arguments)?)?;
            let event = queries::insert_event(calendar(ctx)?, dto).map_err(|e| ToolError::Failed(e.to_string()))?;
            Ok(json!({ "created": event_json(&event) }))
        }
        "list_notes" => {
            let cards = card_manager::get_all_cards().map_err(ToolError::Failed)?;
            let notes: Vec<Value> = cards
                .iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "title": card_manager::extract_title_from_content(&c.content),
                        "updated": Local.timestamp_opt(c.updated_at, 0).single().map(|t| t.format("%Y-%m-%d").to_string()),
                    })
                })
                .collect();
            Ok(json!({ "notes": notes }))
        }
        "read_note" => {
            let a: ReadNoteArgs = args(arguments)?;
            let cards = card_manager::get_all_cards().map_err(ToolError::Failed)?;
            let card = cards
                .iter()
                .find(|c| c.id == a.id.trim())
                .ok_or_else(|| ToolError::InvalidArguments(format!("no note with id {}", a.id)))?;
            Ok(json!({ "id": card.id, "content": card.content }))
        }
        "create_note" => {
            let a: CreateNoteArgs = args(arguments)?;
            if a.content.trim().is_empty() {
                return Err(ToolError::InvalidArguments("content is empty".to_string()));
            }
            let card = card_manager::create_card(a.content).map_err(ToolError::Failed)?;
            Ok(json!({
                "created": { "id": card.id, "title": card_manager::extract_title_from_content(&card.content) }
            }))
        }
        "list_time_entries" => {
            let (from, to) = day_range(&args(arguments)?)?;
            let query = [("from", from.to_rfc3339()), ("to", to.to_rfc3339())];
            let entries = hextime(ctx, reqwest::Method::GET, "/entries", &query, None).await?;
            let projects = projects(ctx).await?;
            let entries: Vec<Value> = entries
                .as_array()
                .map(|list| list.iter().map(|e| entry_json(e, &projects)).collect())
                .unwrap_or_default();
            let total: i64 = entries.iter().filter_map(|e| e["duration_minutes"].as_i64()).sum();
            Ok(json!({ "entries": entries, "total_minutes": total }))
        }
        "get_timer" => {
            let running = hextime(ctx, reqwest::Method::GET, "/timer", &[], None).await?;
            if running.is_null() {
                return Ok(json!({ "running": false }));
            }
            Ok(json!({ "running": true, "entry": entry_json(&running, &projects(ctx).await?) }))
        }
        "start_timer" => {
            let a: StartTimerArgs = args(arguments)?;
            let project_id = match a.project.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
                None => None,
                Some(wanted) => {
                    let projects = projects(ctx).await?;
                    let found = projects
                        .iter()
                        .find(|(_, name)| name.eq_ignore_ascii_case(wanted))
                        .or_else(|| {
                            let wanted = wanted.to_lowercase();
                            projects.iter().find(|(_, name)| name.to_lowercase().contains(&wanted))
                        });
                    match found {
                        Some((id, _)) => Some(id.clone()),
                        None => {
                            let names: Vec<&str> = projects.iter().map(|(_, n)| n.as_str()).collect();
                            return Err(ToolError::InvalidArguments(format!(
                                "no project named '{}'. Existing projects: {}",
                                wanted,
                                if names.is_empty() { "none".to_string() } else { names.join(", ") }
                            )));
                        }
                    }
                }
            };
            let body = json!({ "description": a.description, "project_id": project_id });
            let entry = hextime(ctx, reqwest::Method::POST, "/timer/start", &[], Some(body)).await?;
            Ok(json!({ "started": entry_json(&entry, &projects(ctx).await?) }))
        }
        "stop_timer" => {
            let entry = hextime(ctx, reqwest::Method::POST, "/timer/stop", &[], None).await?;
            Ok(json!({ "stopped": entry_json(&entry, &projects(ctx).await?) }))
        }
        other => Err(ToolError::UnknownTool(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_args(value: Value) -> CreateEventArgs {
        args(&value).unwrap()
    }

    #[test]
    fn every_tool_has_a_valid_definition() {
        let defs = definitions();
        assert_eq!(defs.len(), TOOLS.len());
        for def in defs {
            assert_eq!(def["type"], "function");
            assert_eq!(def["function"]["parameters"]["type"], "object");
        }
    }

    #[test]
    fn classifies_read_and_write_tools() {
        assert_eq!(kind("list_events").unwrap(), ToolKind::Read);
        assert_eq!(kind("create_event").unwrap(), ToolKind::Write);
        assert_eq!(kind("start_timer").unwrap(), ToolKind::Write);
        assert!(matches!(kind("delete_everything"), Err(ToolError::UnknownTool(_))));
    }

    #[test]
    fn builds_timed_all_day_and_default_length_events() {
        let timed = event_dto(&event_args(json!({
            "title": " Dentist ", "date": "2026-10-05", "start_time": "10:00", "end_time": "11:30"
        })))
        .unwrap();
        assert_eq!(timed.title, "Dentist");
        assert_eq!(timed.all_day, Some(false));
        assert_eq!(timed.end_ts - timed.start_ts, 90 * 60 * 1000);
        assert_eq!(local_from_millis(timed.start_ts).unwrap().format("%Y-%m-%d %H:%M").to_string(), "2026-10-05 10:00");

        let all_day = event_dto(&event_args(json!({ "title": "Holiday", "date": "2026-10-05" }))).unwrap();
        assert_eq!(all_day.all_day, Some(true));
        assert_eq!(local_from_millis(all_day.start_ts).unwrap().format("%H:%M").to_string(), "00:00");

        let one_hour = event_dto(&event_args(json!({ "title": "Call", "date": "2026-10-05", "start_time": "9:15" }))).unwrap();
        assert_eq!(one_hour.end_ts - one_hour.start_ts, 60 * 60 * 1000);
    }

    #[test]
    fn rejects_bad_event_arguments() {
        for bad in [
            json!({ "title": "", "date": "2026-10-05" }),
            json!({ "title": "X", "date": "5.10.2026" }),
            json!({ "title": "X", "date": "2026-10-05", "start_time": "25:00" }),
            json!({ "title": "X", "date": "2026-10-05", "start_time": "10:00", "end_time": "09:00" }),
        ] {
            assert!(event_dto(&event_args(bad.clone())).is_err(), "{bad}");
        }
        assert!(args::<CreateEventArgs>(&json!({ "date": "2026-10-05" })).is_err());
    }

    #[test]
    fn day_range_is_inclusive_and_validated() {
        let (from, to) = day_range(&DateRangeArgs { from_date: "2026-10-05".into(), to_date: Some("2026-10-11".into()) }).unwrap();
        assert_eq!(from.format("%Y-%m-%d %H:%M").to_string(), "2026-10-05 00:00");
        assert_eq!(to.format("%Y-%m-%d %H:%M").to_string(), "2026-10-12 00:00");

        let (from, to) = day_range(&DateRangeArgs { from_date: "2026-10-05".into(), to_date: None }).unwrap();
        assert_eq!((to - from).num_hours(), 24);

        assert!(day_range(&DateRangeArgs { from_date: "2026-10-05".into(), to_date: Some("2026-10-01".into()) }).is_err());
    }

    #[test]
    fn describes_write_actions_for_confirmation() {
        let text = describe(
            "create_event",
            &json!({ "title": "Dentist", "date": "2026-10-05", "start_time": "10:00", "location": "Kotka" }),
        )
        .unwrap();
        assert_eq!(text, "Create event “Dentist” on Mon 5 Oct 2026, 10:00–11:00 at Kotka");

        let note = describe("create_note", &json!({ "content": "# Week summary\n\n- a" })).unwrap();
        assert_eq!(note, "Create note “Week summary”");

        let timer = describe("start_timer", &json!({ "description": "Coding", "project": "HexTime" })).unwrap();
        assert_eq!(timer, "Start timer: Coding on HexTime");
        assert_eq!(describe("stop_timer", &Value::Null).unwrap(), "Stop the running timer");
    }

    #[tokio::test]
    async fn creates_and_lists_calendar_events() {
        let db = DbPool::open_in_memory().unwrap();
        let hextime = HexTime::default();
        let client = Client::new();
        let ctx = ToolContext { calendar: Some(&db), hextime: &hextime, client: &client };

        execute(&ctx, "create_event", &json!({ "title": "Standup", "date": "2026-10-05", "start_time": "09:00" }))
            .await
            .unwrap();
        execute(&ctx, "create_event", &json!({ "title": "Next week", "date": "2026-10-13" }))
            .await
            .unwrap();

        let week = execute(&ctx, "list_events", &json!({ "from_date": "2026-10-05", "to_date": "2026-10-11" }))
            .await
            .unwrap();
        let events = week["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["title"], "Standup");
        assert_eq!(events[0]["start"], "09:00");
        assert_eq!(events[0]["end"], "10:00");
    }

    /// Needs the sidecar (sidecar/hextime/build.sh); uses a throwaway HexTime database
    #[tokio::test]
    #[ignore]
    async fn tracks_time_through_hextime() {
        let db_file = std::env::temp_dir().join(format!("hextime-tools-test-{}.db", std::process::id()));
        std::env::set_var("DATABASE_URL", format!("sqlite+pysqlite:///{}", db_file.display()));

        let hextime = HexTime::default();
        let client = Client::new();
        let ctx = ToolContext { calendar: None, hextime: &hextime, client: &client };

        assert_eq!(execute(&ctx, "get_timer", &Value::Null).await.unwrap()["running"], false);

        let started = execute(&ctx, "start_timer", &json!({ "description": "Writing tests" })).await.unwrap();
        assert_eq!(started["started"]["description"], "Writing tests");
        assert_eq!(execute(&ctx, "get_timer", &Value::Null).await.unwrap()["running"], true);

        let unknown_project = execute(&ctx, "start_timer", &json!({ "project": "Nope" })).await;
        assert!(matches!(unknown_project, Err(ToolError::InvalidArguments(_))));

        let stopped = execute(&ctx, "stop_timer", &Value::Null).await.unwrap();
        assert_eq!(stopped["stopped"]["running"], false);
        assert!(matches!(execute(&ctx, "stop_timer", &Value::Null).await, Err(ToolError::Failed(_))));

        let today = Local::now().format("%Y-%m-%d").to_string();
        let listed = execute(&ctx, "list_time_entries", &json!({ "from_date": today })).await.unwrap();
        assert_eq!(listed["entries"].as_array().unwrap().len(), 1);
        assert_eq!(listed["entries"][0]["description"], "Writing tests");

        hextime.shutdown();
        std::env::remove_var("DATABASE_URL");
        let _ = std::fs::remove_file(&db_file);
    }

    #[tokio::test]
    async fn reports_a_missing_calendar() {
        let hextime = HexTime::default();
        let client = Client::new();
        let ctx = ToolContext { calendar: None, hextime: &hextime, client: &client };
        let result = execute(&ctx, "list_events", &json!({ "from_date": "2026-10-05" })).await;
        assert!(matches!(result, Err(ToolError::Unavailable(_))));
    }
}
