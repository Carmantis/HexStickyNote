import { invoke } from '@tauri-apps/api/core';
import { fromIsoDate } from '../utils/dateHelpers';
import type {
  CalendarData,
  CalendarEvent,
  CreateEventDto,
  CreateReminderDto,
  DayNotes,
  Reminder,
  UpdateEventDto,
  ViewMode,
} from '../types';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let _events = $state<CalendarEvent[]>([]);
let _notes = $state<DayNotes[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);

// ---------------------------------------------------------------------------
// Derived
// ---------------------------------------------------------------------------

export const events = {
  get current() { return _events; },
};

export const notes = {
  get current() { return _notes; },
};

export const loading = {
  get current() { return _loading; },
};

export const error = {
  get current() { return _error; },
};

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadCalendarData(mode: ViewMode, anchorDate: string): Promise<void> {
  _loading = true;
  _error = null;
  try {
    const data = await invoke<CalendarData>('get_calendar_data', {
      mode,
      anchorDate: anchorDate,
    });
    _events = data.events;
    _notes = data.notes;
  } catch (e) {
    _error = String(e);
  } finally {
    _loading = false;
  }
}

export async function createEvent(dto: CreateEventDto): Promise<CalendarEvent | null> {
  try {
    const event = await invoke<CalendarEvent>('create_event', { payload: dto });
    _events = [..._events, event];
    return event;
  } catch (e) {
    _error = String(e);
    return null;
  }
}

export async function updateEvent(dto: UpdateEventDto): Promise<CalendarEvent | null> {
  try {
    const updated = await invoke<CalendarEvent>('update_event', { payload: dto });
    _events = _events.map((e) => (e.id === updated.id ? updated : e));
    return updated;
  } catch (e) {
    _error = String(e);
    return null;
  }
}

export async function deleteEvent(id: string): Promise<boolean> {
  try {
    const target = _events.find((e) => e.id === id);
    await invoke<void>('delete_event', { id });
    _events = _events.filter((e) => e.id !== id);
    return true;
  } catch (e) {
    _error = String(e);
    return false;
  }
}

export async function getDayNotes(date: string): Promise<DayNotes | null> {
  try {
    return await invoke<DayNotes>('get_day_notes', { date });
  } catch (e) {
    _error = String(e);
    return null;
  }
}

export async function saveDayNotes(date: string, content: string): Promise<DayNotes | null> {
  try {
    const saved = await invoke<DayNotes>('save_day_notes', { date, content });
    _notes = _notes.map((n) => (n.date === saved.date ? saved : n));
    if (!_notes.some((n) => n.date === saved.date)) {
      _notes = [..._notes, saved];
    }
    return saved;
  } catch (e) {
    _error = String(e);
    return null;
  }
}

export async function scheduleReminder(dto: CreateReminderDto): Promise<Reminder | null> {
  try {
    return await invoke<Reminder>('schedule_reminder', { payload: dto });
  } catch (e) {
    _error = String(e);
    return null;
  }
}

export async function dismissReminder(reminderId: string): Promise<void> {
  try {
    await invoke<void>('dismiss_reminder', { reminderId: reminderId });
  } catch (e) {
    _error = String(e);
  }
}

/** Returns all events for a specific ISO date. */
export function eventsForDate(isoDate: string): CalendarEvent[] {
  const dayStart = fromIsoDate(isoDate).getTime();
  const dayEnd = dayStart + 86_400_000;
  return _events.filter((e) => e.start_ts < dayEnd && e.end_ts > dayStart);
}

/** Returns the notes entry for a specific ISO date, or null. */
export function notesForDate(isoDate: string): DayNotes | null {
  return _notes.find((n) => n.date === isoDate) ?? null;
}
