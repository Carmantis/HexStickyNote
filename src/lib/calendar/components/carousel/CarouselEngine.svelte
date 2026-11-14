<svelte:options runes={true} />

<script lang="ts">
  import type { CalendarEvent } from '$lib/calendar/types';
  import {
    transform,
    viewMode,
    isDragging,
    transitioning,
    slides,
    onDragStart,
    onDragMove,
    onDragEnd,
  } from '$lib/calendar/stores/carousel.svelte';
  import { gestureAction } from '$lib/calendar/utils/gestures';
  import MonthView from './MonthView.svelte';
  import WeekView from './WeekView.svelte';
  import DayView from './DayView.svelte';

  interface Props {
    onEventClick?: (event: CalendarEvent) => void;
    onDateClick?: (date: string) => void;
    onSlotClick?: (date: string, hour: number) => void;
  }

  let { onEventClick, onDateClick, onSlotClick }: Props = $props();

  const mode = $derived(viewMode.current);
  const currentSlides = $derived(slides.current);
  const isTransitioning = $derived(transitioning.current);
  const dragging = $derived(isDragging.current);
</script>

<div
  class="carousel-viewport"
  use:gestureAction={{ onDragStart, onDragMove, onDragEnd }}
>
  <div
    class="carousel-track"
    class:transitioning={isTransitioning && !dragging}
    style:transform={transform.current}
  >
    {#each currentSlides as slide, i (slide.anchor_date + i)}
      <div class="carousel-slide">
        {#if slide.loading || !slide.data}
          <div class="slide-loading">
            <div class="spinner"></div>
          </div>
        {:else if mode === 'month'}
          <MonthView
            data={slide.data}
            {onDateClick}
            {onEventClick}
          />
        {:else if mode === 'week'}
          <WeekView
            data={slide.data}
            {onEventClick}
            onSlotClick={(date, hour) => onSlotClick?.(date, hour)}
          />
        {:else}
          <DayView
            data={slide.data}
            {onEventClick}
            onSlotClick={(hour) => onSlotClick?.(slide.anchor_date, hour)}
          />
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .carousel-viewport {
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
    cursor: grab;
  }

  .carousel-viewport:active {
    cursor: grabbing;
  }

  .carousel-track {
    display: flex;
    width: 300%;
    height: 100%;
    will-change: transform;
  }

  .carousel-track.transitioning {
    transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .carousel-slide {
    width: calc(100% / 3);
    height: 100%;
    flex-shrink: 0;
    overflow: hidden;
  }

  .slide-loading {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
