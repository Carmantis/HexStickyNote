<svelte:options runes={true} />

<script lang="ts">
  /**
   * Calendar view (HexCalendar), shown inside the HUD.
   * The AI model comes from HexStickyNote's settings, so the standalone
   * app's settings panel is not included.
   */
  import { onMount } from 'svelte';
  import type { CalendarEvent, ViewMode } from '$lib/calendar/types';
  import {
    initialise,
    switchView,
    navigateNext,
    navigatePrev,
    jumpToDate,
    currentAnchor,
    viewMode,
  } from '$lib/calendar/stores/carousel.svelte';
  import { todayIso, finnishMonth, fromIsoDate } from '$lib/calendar/utils/dateHelpers';
  import CarouselEngine from '$lib/calendar/components/carousel/CarouselEngine.svelte';
  import ViewSwitcher from '$lib/calendar/components/ui/ViewSwitcher.svelte';
  import EventModal from '$lib/calendar/components/ui/EventModal.svelte';
  import AlertToast from '$lib/calendar/components/ui/AlertToast.svelte';
  import '$lib/calendar/calendar.css';

  // Modal/panel state
  let showEventModal = $state(false);
  let editingEvent = $state<CalendarEvent | null>(null);
  let newEventDate = $state('');
  let newEventHour = $state(9);

  const anchor = $derived(currentAnchor.current);
  const mode = $derived(viewMode.current);

  // Compute header title from anchor date
  const headerTitle = $derived(() => {
    if (!anchor) return '';
    const d = fromIsoDate(anchor);
    if (mode === 'day') {
      return `${d.getDate()} ${finnishMonth(d.getMonth())} ${d.getFullYear()}`;
    }
    if (mode === 'week') {
      return `${finnishMonth(d.getMonth())} ${d.getFullYear()}`;
    }
    return `${finnishMonth(d.getMonth())} ${d.getFullYear()}`;
  });

  onMount(() => {
    initialise();
  });

  function handleViewSwitch(m: ViewMode) {
    switchView(m);
  }

  function handleEventClick(event: CalendarEvent) {
    editingEvent = event;
    showEventModal = true;
  }

  function handleDateClick(date: string) {
    if (mode === 'month') {
      jumpToDate(date);
      switchView('day');
    }
  }

  function handleSlotClick(date: string, hour: number) {
    newEventDate = date;
    newEventHour = hour;
    editingEvent = null;
    showEventModal = true;
  }

  function handleNewEvent() {
    newEventDate = anchor ?? todayIso();
    newEventHour = 9;
    editingEvent = null;
    showEventModal = true;
  }

  function closeModal() {
    showEventModal = false;
    editingEvent = null;
  }
</script>

<div class="hexcal">
  <div class="app-layout">
    <!-- Top toolbar -->
    <header class="toolbar">
      <div class="toolbar-left">
        <button class="nav-btn" onclick={navigatePrev} aria-label="Previous">‹</button>
        <button class="nav-btn" onclick={navigateNext} aria-label="Next">›</button>
        <button class="today-btn" onclick={() => jumpToDate(todayIso())}>Today</button>
        <h1 class="header-title">{headerTitle()}</h1>
      </div>

      <div class="toolbar-center">
        <ViewSwitcher active={mode} onSwitch={handleViewSwitch} />
      </div>

      <div class="toolbar-right">
        <button class="new-event-btn" onclick={handleNewEvent}>+ New event</button>
      </div>
    </header>

    <!-- Main area -->
    <div class="main-area">
      <CarouselEngine
        onEventClick={handleEventClick}
        onDateClick={handleDateClick}
        onSlotClick={handleSlotClick}
      />
    </div>
  </div>

  <!-- Modals -->
  {#if showEventModal}
    <EventModal
      event={editingEvent}
      defaultDate={newEventDate}
      defaultHour={newEventHour}
      onClose={closeModal}
    />
  {/if}

  <!-- Global reminder toast -->
  <AlertToast />
</div>

<style>
  .hexcal {
    height: 100%;
  }

  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--color-bg);
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background-color: var(--color-surface);
    flex-shrink: 0;
    gap: var(--space-3);
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
  }

  .toolbar-center {
    flex-shrink: 0;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    justify-content: flex-end;
  }

  .nav-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: none;
    border: 1px solid var(--color-border);
    color: var(--color-text);
    font-size: 1.25rem;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .nav-btn:hover {
    background-color: var(--color-surface-hover);
  }

  .today-btn {
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    background: none;
    border: 1px solid var(--color-border);
    color: var(--color-text);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .today-btn:hover {
    background-color: var(--color-surface-hover);
  }

  .header-title {
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--color-text);
    margin-left: var(--space-2);
    text-transform: capitalize;
  }




  .new-event-btn {
    padding: var(--space-1) var(--space-3);
    background-color: var(--color-accent);
    color: var(--color-on-accent);
    border: none;
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .new-event-btn:hover {
    opacity: 0.9;
  }

  .main-area {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
</style>
