use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
    pub all_day: bool,
    pub location: Option<String>,
    pub color: String,
    pub recurrence: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventDto {
    pub title: String,
    pub description: Option<String>,
    pub start_ts: i64,
    pub end_ts: i64,
    pub all_day: Option<bool>,
    pub location: Option<String>,
    pub color: Option<String>,
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventDto {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub all_day: Option<bool>,
    pub location: Option<String>,
    pub color: Option<String>,
    pub recurrence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub event_id: String,
    pub offset_min: i64,
    pub notified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReminderDto {
    pub event_id: String,
    pub offset_min: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayNotes {
    pub id: Option<String>,
    pub date: String,
    pub content: Option<String>,
    pub ai_summary: Option<String>,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Month,
    Week,
    Day,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarData {
    pub mode: ViewMode,
    pub anchor_date: String,
    pub events: Vec<Event>,
    pub notes: Vec<DayNotes>,
}
