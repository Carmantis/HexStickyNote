<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarEvent } from '$lib/calendar/types';
  import { formatTime } from '$lib/calendar/utils/dateHelpers';

  interface Props {
    event: CalendarEvent;
    compact?: boolean;
    onClick?: (event: CalendarEvent) => void;
  }

  let { event, compact = false, onClick }: Props = $props();
</script>

<button
  class="event-chip"
  class:compact
  style:--chip-color={event.color}
  onclick={() => onClick?.(event)}
  title={event.title}
>
  {#if !compact && !event.all_day}
    <span class="time">{formatTime(event.start_ts)}</span>
  {/if}
  <span class="title">{event.title}</span>
</button>

<style>
  .event-chip {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: 100%;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background-color: color-mix(in srgb, var(--chip-color) 20%, transparent);
    border-left: 3px solid var(--chip-color);
    cursor: pointer;
    font-size: var(--text-xs);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    transition: opacity 0.15s ease;
    border-top: none;
    border-right: none;
    border-bottom: none;
  }

  .event-chip:hover {
    opacity: 0.85;
  }

  .time {
    color: var(--chip-color);
    font-weight: 600;
    flex-shrink: 0;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-text);
  }

  .compact {
    padding: 1px 4px;
    font-size: 10px;
  }
</style>
