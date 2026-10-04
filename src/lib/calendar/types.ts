// ---------------------------------------------------------------------------
// Shared TypeScript types — all stores and components import from here.
// Do NOT redeclare these interfaces in individual files.
// ---------------------------------------------------------------------------

export interface CalendarEvent {
  id: string;
  title: string;
  description: string | null;
  start_ts: number; // Unix ms
  end_ts: number;   // Unix ms
  all_day: boolean;
  location: string | null;
  color: string;
  recurrence: string | null; // JSON-encoded RRULE object
  source: string;
}

export interface CreateEventDto {
  title: string;
  description?: string;
  start_ts: number;
  end_ts: number;
  all_day?: boolean;
  location?: string;
  color?: string;
  recurrence?: string;
}

export interface UpdateEventDto {
  id: string;
  title?: string;
  description?: string;
  start_ts?: number;
  end_ts?: number;
  all_day?: boolean;
  location?: string;
  color?: string;
  recurrence?: string;
}

export interface DayNotes {
  id: string | null;
  date: string; // ISO 8601: YYYY-MM-DD
  content: string | null;
  ai_summary: string | null;
  updated_at: number | null;
}

export type ViewMode = 'month' | 'week' | 'day';

export interface CalendarData {
  mode: ViewMode;
  anchor_date: string;
  events: CalendarEvent[];
  notes: DayNotes[];
}

export interface Reminder {
  id: string;
  event_id: string;
  offset_min: number;
  notified: boolean;
}

export interface CreateReminderDto {
  event_id: string;
  offset_min: number;
}

export interface AiDigest {
  id: string;
  period_type: string;
  period_key: string;
  content: string;
  model: string | null;
  created_at: number;
}

// Carousel slide wraps calendar data for a single time window
export interface CarouselSlide {
  anchor_date: string;
  data: CalendarData | null;
  loading: boolean;
}

export interface CalError {
  kind: 'Database' | 'NotFound' | 'Ai' | 'Validation' | 'Io' | 'Serialization';
  message: string;
}
