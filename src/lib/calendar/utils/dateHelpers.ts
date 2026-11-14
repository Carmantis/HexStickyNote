// ---------------------------------------------------------------------------
// Date helpers — no external date libraries.
// Uses native Date for compatibility; Temporal API where available.
// ---------------------------------------------------------------------------

export const MS_PER_DAY = 86_400_000;
export const MS_PER_HOUR = 3_600_000;
export const MS_PER_MINUTE = 60_000;

/** Format a Unix-ms timestamp to HH:MM (local time). */
export function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString('default', { hour: '2-digit', minute: '2-digit', hour12: false });
}

/** Format a Unix-ms timestamp to a full date string. */
export function formatDate(ts: number, locale = 'fi-FI'): string {
  return new Date(ts).toLocaleDateString(locale, {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

/** Format a Unix-ms timestamp to short date (e.g. "25.3.2025"). */
export function formatShortDate(ts: number, locale = 'fi-FI'): string {
  return new Date(ts).toLocaleDateString(locale);
}

/** Returns ISO date string YYYY-MM-DD for a Date object (local time). */
export function toIsoDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Parses an ISO date string to a Date at midnight local time. */
export function fromIsoDate(iso: string): Date {
  const [y, m, d] = iso.split('-').map(Number);
  return new Date(y, m - 1, d);
}

/** Returns today's ISO date string. */
export function todayIso(): string {
  return toIsoDate(new Date());
}

/**
 * Returns the ISO date of the Monday of the week containing `date`.
 */
export function weekStart(date: Date): Date {
  const d = new Date(date);
  const dow = (d.getDay() + 6) % 7; // Monday = 0
  d.setDate(d.getDate() - dow);
  d.setHours(0, 0, 0, 0);
  return d;
}

/** Returns ISO week key like "2025-W12". */
export function isoWeekKey(date: Date): string {
  // Compute ISO week number
  const tmp = new Date(date.valueOf());
  tmp.setHours(0, 0, 0, 0);
  tmp.setDate(tmp.getDate() + 3 - ((tmp.getDay() + 6) % 7));
  const week1 = new Date(tmp.getFullYear(), 0, 4);
  const weekNum = Math.round(
    ((tmp.valueOf() - week1.valueOf()) / 86_400_000 + ((week1.getDay() + 6) % 7)) / 7
  ) + 1;
  return `${tmp.getFullYear()}-W${String(weekNum).padStart(2, '0')}`;
}

/** Add `days` days to a Date, returns a new Date. */
export function addDays(date: Date, days: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

/** Add `months` months to a Date, returns a new Date (clamped to last day). */
export function addMonths(date: Date, months: number): Date {
  const d = new Date(date);
  const targetMonth = d.getMonth() + months;
  d.setMonth(targetMonth);
  return d;
}

/**
 * Returns all the ISO date strings for the 6-row month grid containing `anchorDate`.
 * The grid always starts on Monday and contains 42 cells.
 */
export function monthGridDates(anchorDate: string): string[] {
  const anchor = fromIsoDate(anchorDate);
  const firstOfMonth = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const dow = (firstOfMonth.getDay() + 6) % 7; // Monday = 0
  const gridStart = addDays(firstOfMonth, -dow);

  return Array.from({ length: 42 }, (_, i) => toIsoDate(addDays(gridStart, i)));
}

/** Returns the 7 ISO date strings for the week containing `anchorDate`. */
export function weekDates(anchorDate: string): string[] {
  const anchor = fromIsoDate(anchorDate);
  const monday = weekStart(anchor);
  return Array.from({ length: 7 }, (_, i) => toIsoDate(addDays(monday, i)));
}

/** Returns the Unix-ms timestamp for the start of an ISO date string (local midnight). */
export function isoDateToStartTs(iso: string): number {
  return fromIsoDate(iso).getTime();
}

/** Returns the ISO date string for a Unix-ms timestamp (local time). */
export function tsToIsoDate(ts: number): string {
  return toIsoDate(new Date(ts));
}

/** Returns how many minutes an event spans. */
export function eventDurationMin(startTs: number, endTs: number): number {
  return Math.round((endTs - startTs) / MS_PER_MINUTE);
}

/** Returns number of minutes from midnight for a given timestamp (local time). */
export function minutesFromMidnight(ts: number): number {
  const d = new Date(ts);
  return d.getHours() * 60 + d.getMinutes();
}

/** Returns the anchor date offset by `delta` units for the given view mode. */
export function offsetAnchor(anchor: string, delta: number, mode: 'month' | 'week' | 'day'): string {
  const d = fromIsoDate(anchor);
  switch (mode) {
    case 'day':
      return toIsoDate(addDays(d, delta));
    case 'week':
      return toIsoDate(addDays(d, delta * 7));
    case 'month':
      return toIsoDate(addMonths(d, delta));
  }
}

/** Returns true if the ISO date is today. */
export function isToday(iso: string): boolean {
  return iso === todayIso();
}

/** Returns true if both ISO dates are in the same month. */
export function sameMonth(isoA: string, isoB: string): boolean {
  return isoA.slice(0, 7) === isoB.slice(0, 7);
}

/** Returns the Finnish weekday abbreviation (Ma, Ti, …). */
export function finnishWeekday(iso: string): string {
  const d = fromIsoDate(iso);
  const names = ['Su', 'Ma', 'Ti', 'Ke', 'To', 'Pe', 'La'];
  return names[d.getDay()];
}

/** Returns the Finnish month name. */
export function finnishMonth(month: number): string {
  const names = [
    'Tammikuu', 'Helmikuu', 'Maaliskuu', 'Huhtikuu', 'Toukokuu', 'Kesäkuu',
    'Heinäkuu', 'Elokuu', 'Syyskuu', 'Lokakuu', 'Marraskuu', 'Joulukuu',
  ];
  return names[month];
}
