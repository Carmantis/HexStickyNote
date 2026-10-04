<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarEvent, DayNotes } from '$lib/calendar/types';
  import { formatTime } from '$lib/calendar/utils/dateHelpers';
  import EventChip from './EventChip.svelte';

  interface Props {
    date: string;
    events: CalendarEvent[];
    notes: DayNotes | null;
    onEventClick?: (event: CalendarEvent) => void;
    onNotesChange?: (content: string) => void;
  }

  let { date, events, notes, onEventClick, onNotesChange }: Props = $props();

  let notesContent = $state(notes?.content ?? '');

  $effect(() => {
    notesContent = notes?.content ?? '';
  });

  function handleInput(e: Event) {
    const content = (e.target as HTMLDivElement).innerText;
    notesContent = content;
    onNotesChange?.(content);
  }
</script>

<div class="expanded-cell">
  <div class="events-section">
    {#if events.length === 0}
      <p class="empty-msg">No events</p>
    {:else}
      {#each events as event (event.id)}
        <div class="event-row">
          <div class="event-time">
            {#if event.all_day}
              <span>All day</span>
            {:else}
              <span>{formatTime(event.start_ts)}</span>
              <span class="time-sep">–</span>
              <span>{formatTime(event.end_ts)}</span>
            {/if}
          </div>
          <EventChip {event} onClick={onEventClick} />
        </div>
      {/each}
    {/if}
  </div>

  <div class="notes-section">
    <h4 class="notes-label">Notes</h4>
    <div
      class="notes-editor"
      contenteditable="true"
      role="textbox"
      aria-multiline="true"
      aria-label="Day notes"
      oninput={handleInput}
    >
      {notesContent}
    </div>
  </div>
</div>

<style>
  .expanded-cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    height: 100%;
    overflow: hidden;
  }

  .events-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    overflow-y: auto;
    flex-shrink: 0;
    max-height: 40%;
  }

  .event-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .event-time {
    display: flex;
    gap: 2px;
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
    min-width: 80px;
  }

  .time-sep {
    color: var(--color-text-muted);
  }

  .empty-msg {
    font-size: var(--text-sm);
    color: var(--color-text-muted);
    font-style: italic;
  }

  .notes-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1;
    overflow: hidden;
  }

  .notes-label {
    font-size: var(--text-xs);
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
    line-height: 1.6;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
    outline: none;
  }

  .notes-editor:focus {
    border-color: var(--color-accent);
  }
</style>
