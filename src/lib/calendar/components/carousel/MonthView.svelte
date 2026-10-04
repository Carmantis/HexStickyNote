<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarData, CalendarEvent } from '$lib/calendar/types';
  import { monthGridDates, finnishWeekday, fromIsoDate } from '$lib/calendar/utils/dateHelpers';
  import { notesForDate, eventsForDate } from '$lib/calendar/stores/calendar.svelte';
  import DayCell from '../cells/DayCell.svelte';

  interface Props {
    data: CalendarData;
    onDateClick?: (date: string) => void;
    onEventClick?: (event: CalendarEvent) => void;
  }

  let { data, onDateClick, onEventClick }: Props = $props();

  const gridDates = $derived(monthGridDates(data.anchor_date));

  // Day header labels: Mon–Sun
  const dayHeaders = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  function getEvents(date: string): CalendarEvent[] {
    return data.events.filter((e) => {
      const dayStart = fromIsoDate(date).getTime();
      const dayEnd = dayStart + 86_400_000;
      return e.start_ts < dayEnd && e.end_ts > dayStart;
    });
  }

  function getNotes(date: string) {
    return data.notes.find((n) => n.date === date) ?? null;
  }
</script>

<div class="month-view">
  <!-- Weekday header row -->
  <div class="week-headers">
    {#each dayHeaders as label}
      <div class="week-header">{label}</div>
    {/each}
  </div>

  <!-- 6×7 grid -->
  <div class="grid">
    {#each gridDates as date (date)}
      <DayCell
        {date}
        anchorDate={data.anchor_date}
        events={getEvents(date)}
        notes={getNotes(date)}
        {onDateClick}
        {onEventClick}
      />
    {/each}
  </div>
</div>

<style>
  .month-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0;
  }

  .week-headers {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    padding: 0 var(--space-1);
  }

  .week-header {
    text-align: center;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-text-muted);
    padding: var(--space-1) 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-template-rows: repeat(6, 1fr);
    flex: 1;
    gap: 1px;
    padding: 0 var(--space-1) var(--space-1);
    background-color: var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .grid :global(.day-cell) {
    background-color: var(--color-surface);
  }
</style>
