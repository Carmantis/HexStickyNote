<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarEvent, DayNotes } from '$lib/calendar/types';
  import { isToday, sameMonth, fromIsoDate } from '$lib/calendar/utils/dateHelpers';
  import EventChip from './EventChip.svelte';

  interface Props {
    date: string; // YYYY-MM-DD
    anchorDate: string; // month anchor for grey-out logic
    events: CalendarEvent[];
    notes: DayNotes | null;
    onDateClick?: (date: string) => void;
    onEventClick?: (event: CalendarEvent) => void;
  }

  let { date, anchorDate, events, notes, onDateClick, onEventClick }: Props = $props();

  const MAX_CHIPS = 3;
  const visibleEvents = $derived(events.slice(0, MAX_CHIPS));
  const overflow = $derived(Math.max(0, events.length - MAX_CHIPS));
  const today = $derived(isToday(date));
  const inCurrentMonth = $derived(sameMonth(date, anchorDate));
  const dayNumber = $derived(fromIsoDate(date).getDate());
  const hasNotes = $derived(!!notes?.content);
</script>

<div
  class="day-cell"
  class:today
  class:out-of-month={!inCurrentMonth}
  role="button"
  tabindex="0"
  onclick={() => onDateClick?.(date)}
  onkeydown={(e) => e.key === 'Enter' && onDateClick?.(date)}
>
  <div class="cell-header">
    {#if hasNotes}
      <span class="notes-dot" aria-label="Has notes"></span>
    {/if}
    <span class="day-number">{dayNumber}</span>
  </div>

  <div class="event-list">
    {#each visibleEvents as event (event.id)}
      <EventChip {event} compact onClick={onEventClick} />
    {/each}
    {#if overflow > 0}
      <span class="overflow">+{overflow} more</span>
    {/if}
  </div>
</div>

<style>
  .day-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-1);
    border-radius: var(--radius-sm);
    cursor: pointer;
    min-height: 80px;
    transition: background-color 0.1s ease;
  }

  .day-cell:hover {
    background-color: var(--color-surface-hover);
  }

  .out-of-month {
    opacity: 0.35;
  }

  .cell-header {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    justify-content: flex-end;
  }

  .notes-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background-color: var(--color-accent);
    flex-shrink: 0;
  }

  .day-number {
    font-size: var(--text-sm);
    font-weight: 500;
    line-height: 1.4;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }

  .today {
    background-color: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }

  .today .day-number {
    background-color: var(--color-accent);
    color: var(--color-on-accent);
    font-weight: 700;
  }

  .event-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow: hidden;
  }

  .overflow {
    font-size: 10px;
    color: var(--color-text-muted);
    padding-left: 6px;
  }
</style>
