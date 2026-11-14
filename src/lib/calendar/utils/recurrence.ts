// ---------------------------------------------------------------------------
// Recurrence logic — basic RFC 5545 RRULE expansion.
// Only handles the subset needed for HexCalendar MVP:
//   FREQ=DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, COUNT, UNTIL
// ---------------------------------------------------------------------------

import { addDays, addMonths, fromIsoDate, toIsoDate } from './dateHelpers';
import type { CalendarEvent } from '../types';

export interface Rrule {
  freq: 'DAILY' | 'WEEKLY' | 'MONTHLY' | 'YEARLY';
  interval?: number;
  count?: number;
  until?: string; // ISO 8601 date YYYY-MM-DD
  byday?: string[]; // e.g. ['MO', 'WE', 'FR']
}

const DAY_ABBR: Record<string, number> = {
  SU: 0, MO: 1, TU: 2, WE: 3, TH: 4, FR: 5, SA: 6,
};

/**
 * Expand a recurring event into individual occurrences within [rangeStart, rangeEnd].
 * Returns an array of synthetic CalendarEvent instances (cloned with adjusted timestamps).
 */
export function expandRecurrence(
  event: CalendarEvent,
  rangeStart: Date,
  rangeEnd: Date
): CalendarEvent[] {
  if (!event.recurrence) return [];

  let rule: Rrule;
  try {
    rule = JSON.parse(event.recurrence) as Rrule;
  } catch {
    return [];
  }

  const interval = rule.interval ?? 1;
  const duration = event.end_ts - event.start_ts;
  const until = rule.until ? fromIsoDate(rule.until) : null;

  const occurrences: CalendarEvent[] = [];
  let cursor = new Date(event.start_ts);
  let count = 0;
  const maxCount = rule.count ?? 500;

  while (count < maxCount) {
    if (until && cursor > until) break;
    if (cursor >= rangeEnd) break;

    if (cursor >= rangeStart) {
      // Filter by BYDAY if present
      if (rule.byday && rule.byday.length > 0) {
        const dayNum = cursor.getDay();
        const matches = rule.byday.some((d) => DAY_ABBR[d] === dayNum);
        if (matches) {
          occurrences.push(makeOccurrence(event, cursor.getTime(), duration));
        }
      } else {
        occurrences.push(makeOccurrence(event, cursor.getTime(), duration));
      }
    }

    cursor = advanceCursor(cursor, rule.freq, interval);
    count++;
  }

  return occurrences;
}

function makeOccurrence(base: CalendarEvent, startTs: number, duration: number): CalendarEvent {
  return {
    ...base,
    id: `${base.id}_${startTs}`,
    start_ts: startTs,
    end_ts: startTs + duration,
  };
}

function advanceCursor(date: Date, freq: Rrule['freq'], interval: number): Date {
  switch (freq) {
    case 'DAILY':
      return addDays(date, interval);
    case 'WEEKLY':
      return addDays(date, interval * 7);
    case 'MONTHLY':
      return addMonths(date, interval);
    case 'YEARLY':
      return addMonths(date, interval * 12);
  }
}

/**
 * Merge base events and expanded recurring occurrences for a time range.
 */
export function resolveEvents(
  events: CalendarEvent[],
  rangeStart: Date,
  rangeEnd: Date
): CalendarEvent[] {
  const result: CalendarEvent[] = [];

  for (const event of events) {
    if (event.recurrence) {
      result.push(...expandRecurrence(event, rangeStart, rangeEnd));
    } else {
      result.push(event);
    }
  }

  return result.sort((a, b) => a.start_ts - b.start_ts);
}

/** Parse a raw RRULE JSON string safely. Returns null on failure. */
export function parseRrule(raw: string | null): Rrule | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw) as Rrule;
  } catch {
    return null;
  }
}

/** Serialize an Rrule object to a JSON string for storage. */
export function serializeRrule(rule: Rrule): string {
  return JSON.stringify(rule);
}

/** Human-readable description of a recurrence rule. */
export function rruleDescription(rule: Rrule): string {
  const interval = rule.interval ?? 1;

  const freqLabel: Record<Rrule['freq'], string> = {
    DAILY: interval === 1 ? 'Daily' : `Every ${interval} days`,
    WEEKLY: interval === 1 ? 'Weekly' : `Every ${interval} weeks`,
    MONTHLY: interval === 1 ? 'Monthly' : `Every ${interval} months`,
    YEARLY: interval === 1 ? 'Yearly' : `Every ${interval} years`,
  };

  let desc = freqLabel[rule.freq];

  if (rule.byday && rule.byday.length > 0) {
    desc += ` on ${rule.byday.join(', ')}`;
  }

  if (rule.until) {
    desc += ` until ${rule.until}`;
  } else if (rule.count) {
    desc += `, ${rule.count} times`;
  }

  return desc;
}
