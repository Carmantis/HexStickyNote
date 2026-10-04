<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarData, CalendarEvent } from '$lib/calendar/types';
  import {
    formatDate,
    formatTime,
    fromIsoDate,
    isToday,
    minutesFromMidnight,
  } from '$lib/calendar/utils/dateHelpers';
  import { saveDayNotes } from '$lib/calendar/stores/calendar.svelte';
  import { patchNote } from '$lib/calendar/stores/carousel.svelte';

  // Static data loaded at build time
  import holidays from '$lib/calendar/data/holidays.json';
  import namedays from '$lib/calendar/data/namedays.json';

  interface Props {
    data: CalendarData;
    onEventClick?: (event: CalendarEvent) => void;
    onSlotClick?: (hour: number) => void;
  }

  let { data, onEventClick, onSlotClick }: Props = $props();

  const SLOT_HEIGHT_PX = 48;
  const date = $derived(data.anchor_date);

  const timeSlots = Array.from({ length: 48 }, (_, i) => {
    const h = Math.floor(i / 2);
    const m = i % 2 === 0 ? '00' : '30';
    return { label: `${String(h).padStart(2, '0')}:${m}`, hour: h, half: i % 2 !== 0 };
  });

  const timedEvents = $derived(
    data.events
      .filter((e) => !e.all_day)
      .sort((a, b) => a.start_ts - b.start_ts)
  );

  const allDayEvents = $derived(data.events.filter((e) => e.all_day));

  const notesForDay = $derived(data.notes.find((n) => n.date === date) ?? null);

  const holiday = $derived(
    (holidays as Record<string, string>)[date] ?? null
  );

  const nameday = $derived(
    (namedays as Record<string, string>)[date.slice(5)] ?? null // MM-DD key
  );

  const dayTitle = $derived(
    formatDate(fromIsoDate(date).getTime())
  );

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let notesEl: HTMLDivElement | null = null;

  // Update editor content only when the loaded note changes (day switch or external save),
  // NOT on every keystroke — otherwise Svelte re-renders the div and resets the cursor.
  $effect(() => {
    const content = notesForDay?.content ?? '';
    if (notesEl && notesEl.innerText !== content) {
      notesEl.innerText = content;
    }
  });

  function handleNotesInput(e: Event) {
    const content = (e.target as HTMLDivElement).innerText;

    // Debounce save by 800ms
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const saved = await saveDayNotes(date, content);
      if (saved) patchNote(saved);
    }, 800);
  }

  function eventStyle(event: CalendarEvent): string {
    const dayStart = fromIsoDate(date).getTime();
    const startMin = minutesFromMidnight(Math.max(event.start_ts, dayStart));
    const endMin = minutesFromMidnight(
      event.end_ts > dayStart + 86_400_000 ? dayStart + 86_400_000 : event.end_ts
    );
    const top = (startMin / 30) * SLOT_HEIGHT_PX;
    const height = Math.max(((endMin - startMin) / 30) * SLOT_HEIGHT_PX, SLOT_HEIGHT_PX);
    return `top: ${top}px; height: ${height}px;`;
  }
</script>

<div class="day-view">
  <!-- Header -->
  <header class="day-header">
    <div class="day-meta">
      <h2 class="day-title" class:today={isToday(date)}>{dayTitle}</h2>
      <div class="day-info">
        {#if holiday}
          <span class="holiday">{holiday}</span>
        {/if}
        {#if nameday}
          <span class="nameday">Name day: {nameday}</span>
        {/if}
      </div>
    </div>
  </header>

  <div class="day-body">
    <!-- Timetable -->
    <div class="timetable">
      {#if allDayEvents.length > 0}
        <div class="all-day-strip">
          {#each allDayEvents as event (event.id)}
            <button
              class="all-day-pill"
              style:background-color={event.color}
              onclick={() => onEventClick?.(event)}
            >{event.title}</button>
          {/each}
        </div>
      {/if}

      <div class="time-grid">
        <div class="time-labels">
          {#each timeSlots as slot}
            {#if !slot.half}
              <div class="time-label" style:height="{SLOT_HEIGHT_PX * 2}px">{slot.label}</div>
            {/if}
          {/each}
        </div>

        <div class="slots-col">
          {#each timeSlots as slot, i}
            <div
              class="slot"
              class:half={slot.half}
              style:height="{SLOT_HEIGHT_PX}px"
              role="button"
              tabindex="0"
              onclick={() => !slot.half && onSlotClick?.(slot.hour)}
              onkeydown={(e) => e.key === 'Enter' && !slot.half && onSlotClick?.(slot.hour)}
            ></div>
          {/each}

          {#each timedEvents as event (event.id)}
            <button
              class="event-block"
              style="{eventStyle(event)} border-left-color: {event.color}; background-color: color-mix(in srgb, {event.color} 15%, var(--color-surface));"
              onclick={() => onEventClick?.(event)}
              title={event.title}
            >
              <span class="event-title">{event.title}</span>
              <span class="event-time">{formatTime(event.start_ts)} – {formatTime(event.end_ts)}</span>
              {#if event.location}
                <span class="event-location">{event.location}</span>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Notes panel -->
    <div class="notes-panel">
      <h3 class="notes-title">Notes</h3>
      <div
        class="notes-editor"
        contenteditable="true"
        role="textbox"
        aria-multiline="true"
        aria-label="Day notes"
        bind:this={notesEl}
        oninput={handleNotesInput}
      ></div>
    </div>
  </div>
</div>

<style>
  .day-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .day-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    gap: var(--space-4);
  }

  .day-meta {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .day-title {
    font-size: var(--text-xl);
    font-weight: 600;
    margin: 0;
    color: var(--color-text);
    text-transform: capitalize;
  }

  .day-title.today {
    color: var(--color-accent);
  }

  .day-info {
    display: flex;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .holiday {
    color: var(--color-error);
    font-weight: 500;
  }

  .nameday {
    color: var(--color-text-muted);
  }

  .day-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .timetable {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--color-border);
  }

  .all-day-strip {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .all-day-pill {
    font-size: var(--text-xs);
    padding: 2px 8px;
    border-radius: 999px;
    color: white;
    cursor: pointer;
    border: none;
  }

  .time-grid {
    display: flex;
    flex: 1;
    overflow-y: auto;
  }

  .time-labels {
    width: 52px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
  }

  .time-label {
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding-right: var(--space-1);
    font-size: 10px;
    color: var(--color-text-muted);
    transform: translateY(-7px);
  }

  .slots-col {
    flex: 1;
    position: relative;
  }

  .slot {
    border-bottom: 1px solid var(--color-border-light);
    box-sizing: border-box;
    width: 100%;
    cursor: pointer;
  }

  .slot.half {
    border-bottom-style: dashed;
    opacity: 0.4;
  }

  .slot:hover:not(.half) {
    background-color: var(--color-surface-hover);
  }

  .event-block {
    position: absolute;
    left: 4px;
    right: 4px;
    border-left: 3px solid;
    border-radius: var(--radius-sm);
    padding: 3px 6px;
    overflow: hidden;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 1px;
    z-index: 1;
    text-align: left;
    border-top: none;
    border-right: none;
    border-bottom: none;
  }

  .event-block:hover {
    filter: brightness(0.95);
    z-index: 2;
  }

  .event-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .event-time {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }

  .event-location {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    font-style: italic;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .notes-panel {
    width: 280px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: var(--space-3);
    gap: var(--space-2);
    overflow: hidden;
  }

  .notes-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .notes-editor {
    flex: 1;
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background-color: var(--color-surface);
    font-size: var(--text-sm);
    line-height: 1.7;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
    outline: none;
  }

  .notes-editor:focus {
    border-color: var(--color-accent);
  }
</style>
