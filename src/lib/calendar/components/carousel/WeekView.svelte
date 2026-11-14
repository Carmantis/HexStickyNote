<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarData, CalendarEvent } from '$lib/calendar/types';
  import { weekDates, formatTime, isToday, fromIsoDate, minutesFromMidnight, MS_PER_DAY } from '$lib/calendar/utils/dateHelpers';

  interface Props {
    data: CalendarData;
    onEventClick?: (event: CalendarEvent) => void;
    onSlotClick?: (date: string, hour: number) => void;
  }

  let { data, onEventClick, onSlotClick }: Props = $props();

  const days = $derived(weekDates(data.anchor_date));

  // 48 half-hour slots (00:00 – 23:30)
  const timeSlots = Array.from({ length: 48 }, (_, i) => {
    const h = Math.floor(i / 2);
    const m = i % 2 === 0 ? '00' : '30';
    return `${String(h).padStart(2, '0')}:${m}`;
  });

  const SLOT_HEIGHT_PX = 30; // height of each 30-min slot in px
  const GRID_HEIGHT_PX = 48 * SLOT_HEIGHT_PX; // 1440px — explicit height for absolute positioning

  function eventsForDay(date: string): CalendarEvent[] {
    const start = fromIsoDate(date).getTime();
    const end = start + MS_PER_DAY;
    return data.events
      .filter((e) => !e.all_day && e.start_ts < end && e.end_ts > start)
      .sort((a, b) => a.start_ts - b.start_ts);
  }

  function allDayEventsForDay(date: string): CalendarEvent[] {
    const start = fromIsoDate(date).getTime();
    const end = start + MS_PER_DAY;
    return data.events.filter((e) => e.all_day && e.start_ts < end && e.end_ts > start);
  }

  function eventStyle(event: CalendarEvent, date: string): string {
    const dayStart = fromIsoDate(date).getTime();
    const dayEnd = dayStart + MS_PER_DAY;
    const startMin = minutesFromMidnight(Math.max(event.start_ts, dayStart));
    // If event ends at or beyond day boundary treat as 1440 (24:00) to avoid wrapping to 0
    const rawEnd = Math.min(event.end_ts, dayEnd);
    const endMin = rawEnd >= dayEnd ? 1440 : minutesFromMidnight(rawEnd);
    const top = (startMin / 30) * SLOT_HEIGHT_PX;
    const height = Math.max(((endMin - startMin) / 30) * SLOT_HEIGHT_PX, SLOT_HEIGHT_PX);

    return `top: ${top}px; height: ${height}px;`;
  }
</script>

<div class="week-view">
  <!-- All-day row -->
  <div class="all-day-row">
    <div class="time-gutter-spacer"></div>
    {#each days as date (date)}
      <div class="all-day-col">
        {#each allDayEventsForDay(date) as event (event.id)}
          <button
            class="all-day-chip"
            style:background-color={event.color}
            onclick={() => onEventClick?.(event)}
          >{event.title}</button>
        {/each}
      </div>
    {/each}
  </div>

  <!-- Day column headers -->
  <div class="day-headers">
    <div class="time-gutter-spacer"></div>
    {#each days as date (date)}
      <div class="day-header" class:today={isToday(date)}>
        <span class="day-name">{fromIsoDate(date).toLocaleDateString('en', { weekday: 'short' })}</span>
        <span class="day-num">{fromIsoDate(date).getDate()}</span>
      </div>
    {/each}
  </div>

  <!-- Time grid -->
  <div class="grid-scroll">
    <div class="time-gutter">
      {#each timeSlots as slot, i}
        {#if i % 2 === 0}
          <div class="time-label" style:height="{SLOT_HEIGHT_PX * 2}px">{slot}</div>
        {/if}
      {/each}
      <div class="time-label time-label-end">00:00</div>
    </div>

    {#each days as date (date)}
      <div class="day-column" style:height="{GRID_HEIGHT_PX}px">
        <!-- Slot lines -->
        {#each timeSlots as slot, i}
          <div
            class="slot"
            class:half={i % 2 !== 0}
            style:height="{SLOT_HEIGHT_PX}px"
            role="button"
            tabindex="0"
            onclick={() => onSlotClick?.(date, Math.floor(i / 2))}
            onkeydown={(e) => e.key === 'Enter' && onSlotClick?.(date, Math.floor(i / 2))}
          ></div>
        {/each}

        <!-- Events positioned absolutely -->
        {#each eventsForDay(date) as event (event.id)}
          <button
            class="event-block"
            style="{eventStyle(event, date)} background-color: color-mix(in srgb, {event.color} 20%, var(--color-surface)); border-left-color: {event.color};"
            onclick={() => onEventClick?.(event)}
            title={event.title}
          >
            <span class="event-title">{event.title}</span>
            <span class="event-time">{formatTime(event.start_ts)} – {formatTime(event.end_ts)}</span>
          </button>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .week-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .day-headers {
    display: grid;
    grid-template-columns: 48px repeat(7, 1fr);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .time-gutter-spacer {
    width: 48px;
    flex-shrink: 0;
  }

  .day-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-1) 0;
    gap: 2px;
  }

  .day-name {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    text-transform: uppercase;
  }

  .day-num {
    font-size: var(--text-sm);
    font-weight: 500;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }

  .today .day-num {
    background-color: var(--color-accent);
    color: var(--color-on-accent);
    font-weight: 700;
  }

  .all-day-row {
    display: grid;
    grid-template-columns: 48px repeat(7, 1fr);
    min-height: 28px;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .all-day-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px;
  }

  .all-day-chip {
    font-size: var(--text-xs);
    padding: 1px 4px;
    border-radius: 3px;
    color: white;
    cursor: pointer;
    border: none;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .grid-scroll {
    display: flex;
    overflow-y: auto;
    flex: 1;
    align-items: flex-start;
  }

  .time-gutter {
    width: 48px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--color-border);
  }

  .time-label {
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding-right: var(--space-1);
    font-size: 10px;
    color: var(--color-text-muted);
    transform: translateY(-6px);
  }

  .time-label-end {
    border-top: 1px solid var(--color-border);
    transform: none;
    align-items: center;
    height: 16px;
  }

  .day-column {
    flex: 1;
    position: relative;
    border-right: 1px solid var(--color-border);
  }

  .slot {
    border-bottom: 1px solid var(--color-border-light);
    box-sizing: border-box;
    width: 100%;
    cursor: pointer;
  }

  .slot.half {
    border-bottom-style: dashed;
    opacity: 0.5;
  }

  .slot:hover {
    background-color: var(--color-surface-hover);
  }

  .event-block {
    position: absolute;
    left: 2px;
    right: 2px;
    border-left: 3px solid;
    border-radius: var(--radius-sm);
    padding: 2px 4px;
    overflow: hidden;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    font-size: var(--text-xs);
    text-align: left;
    border-top: none;
    border-right: none;
    border-bottom: none;
    z-index: 1;
  }

  .event-block:hover {
    z-index: 2;
    filter: brightness(0.95);
  }

  .event-title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text);
  }

  .event-time {
    font-size: 10px;
    color: var(--color-text-muted);
  }
</style>
