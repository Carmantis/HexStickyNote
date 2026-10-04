import fs from "fs";
import { DatabaseSync } from "node:sqlite";
import { v4 as uuidv4 } from "uuid";
import { getCalendarDatabasePath } from "./paths.js";

// Calendar events in HexStickyNote's calendar (HexCalendar's SQLite database).
// Same rules as the app's assistant tools (src-tauri/src/assistant/tools.rs):
// local dates (YYYY-MM-DD) and times (HH:MM); no start time = all-day event;
// no end time = one hour.

const DEFAULT_COLOR = "#4A90D9";

interface EventRow {
  id: string;
  title: string;
  description: string | null;
  start_ts: number;
  end_ts: number;
  all_day: number;
  location: string | null;
  recurrence: string | null;
}

export interface EventSummary {
  id: string;
  title: string;
  date: string;
  start: string | null;
  end: string | null;
  all_day: boolean;
  location: string | null;
  description: string | null;
  recurring: boolean;
}

export interface NewEvent {
  title: string;
  date: string;
  start_time?: string;
  end_time?: string;
  location?: string;
  description?: string;
}

function openDatabase(): DatabaseSync {
  const file = getCalendarDatabasePath();
  if (!fs.existsSync(file)) {
    throw new Error("The calendar database does not exist yet. Open HexStickyNote once to create it.");
  }
  const db = new DatabaseSync(file);
  // The app may have the database open at the same time
  db.exec("PRAGMA busy_timeout = 5000;");
  return db;
}

function parseDate(value: string): { year: number; month: number; day: number } {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value.trim());
  if (!match) throw new Error(`'${value}' is not a YYYY-MM-DD date`);
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const check = new Date(year, month - 1, day);
  if (check.getFullYear() !== year || check.getMonth() !== month - 1 || check.getDate() !== day) {
    throw new Error(`'${value}' is not a valid date`);
  }
  return { year, month, day };
}

function parseTime(value: string): { hours: number; minutes: number } {
  const match = /^(\d{1,2}):(\d{2})(?::\d{2})?$/.exec(value.trim());
  if (!match) throw new Error(`'${value}' is not an HH:MM time`);
  const [hours, minutes] = [Number(match[1]), Number(match[2])];
  if (hours > 23 || minutes > 59) throw new Error(`'${value}' is not a valid time`);
  return { hours, minutes };
}

/** Local midnight of a date, in milliseconds */
function localDay(date: string, dayOffset = 0): number {
  const { year, month, day } = parseDate(date);
  return new Date(year, month - 1, day + dayOffset).getTime();
}

function localTime(date: string, time: string): number {
  const { year, month, day } = parseDate(date);
  const { hours, minutes } = parseTime(time);
  return new Date(year, month - 1, day, hours, minutes).getTime();
}

const pad = (n: number) => String(n).padStart(2, "0");
const formatDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const formatTime = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

function summarize(row: EventRow): EventSummary {
  const start = new Date(row.start_ts);
  const end = new Date(row.end_ts);
  const allDay = row.all_day !== 0;
  return {
    id: row.id,
    title: row.title,
    date: formatDate(start),
    start: allDay ? null : formatTime(start),
    end: allDay ? null : formatTime(end),
    all_day: allDay,
    location: row.location,
    description: row.description,
    recurring: row.recurrence !== null,
  };
}

/** Events overlapping the days from `fromDate` to `toDate` (inclusive) */
export function listEvents(fromDate: string, toDate?: string): EventSummary[] {
  const start = localDay(fromDate);
  const end = localDay(toDate?.trim() ? toDate : fromDate, 1);
  if (end <= start) throw new Error("to_date is before from_date");

  const db = openDatabase();
  try {
    const rows = db
      .prepare(
        `SELECT id, title, description, start_ts, end_ts, all_day, location, recurrence
         FROM events WHERE start_ts < ? AND end_ts > ? ORDER BY start_ts ASC`
      )
      .all(end, start) as unknown as EventRow[];
    return rows.map(summarize);
  } finally {
    db.close();
  }
}

export function createEvent(event: NewEvent): EventSummary {
  const title = event.title.trim();
  if (!title) throw new Error("title is empty");

  let start: number;
  let end: number;
  let allDay: boolean;
  if (!event.start_time?.trim()) {
    start = localDay(event.date);
    end = localDay(event.date, 1);
    allDay = true;
  } else {
    start = localTime(event.date, event.start_time);
    end = event.end_time?.trim() ? localTime(event.date, event.end_time) : start + 60 * 60 * 1000;
    allDay = false;
    if (end <= start) throw new Error("end_time must be after start_time");
  }

  const row: EventRow = {
    id: uuidv4(),
    title,
    description: event.description?.trim() || null,
    start_ts: start,
    end_ts: end,
    all_day: allDay ? 1 : 0,
    location: event.location?.trim() || null,
    recurrence: null,
  };

  const db = openDatabase();
  try {
    db.prepare(
      `INSERT INTO events (id, title, description, start_ts, end_ts, all_day, location, color, recurrence, source)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, 'local')`
    ).run(row.id, row.title, row.description, row.start_ts, row.end_ts, row.all_day, row.location, DEFAULT_COLOR);
  } finally {
    db.close();
  }
  return summarize(row);
}
