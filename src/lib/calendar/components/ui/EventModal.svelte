<svelte:options runes={true} />

<script lang="ts">
  import { untrack } from 'svelte';
  import type { CalendarEvent, CreateEventDto, UpdateEventDto } from '$lib/calendar/types';
  import { createEvent, updateEvent, deleteEvent } from '$lib/calendar/stores/calendar.svelte';

  interface Props {
    event?: CalendarEvent | null; // null = create mode
    defaultDate?: string;
    defaultHour?: number;
    onClose: () => void;
  }

  let { event = null, defaultDate = '', defaultHour = 9, onClose }: Props = $props();

  const isEditing = $derived(!!event);

  // Form state — untrack() marks the initial prop read as intentional
  // (the modal mounts fresh each time, so one-time capture is correct).
  let title = $state(untrack(() => event?.title ?? ''));
  let description = $state(untrack(() => event?.description ?? ''));
  let startDate = $state(untrack(() => defaultDate || event
    ? new Date(event?.start_ts ?? Date.now()).toISOString().slice(0, 10)
    : new Date().toISOString().slice(0, 10)));
  let startTime = $state(untrack(() => event
    ? new Date(event.start_ts).toTimeString().slice(0, 5)
    : `${String(defaultHour).padStart(2, '0')}:00`));
  let endTime = $state(untrack(() => event
    ? new Date(event.end_ts).toTimeString().slice(0, 5)
    : `${String(defaultHour + 1).padStart(2, '0')}:00`));
  let allDay = $state(untrack(() => event?.all_day ?? false));
  let location = $state(untrack(() => event?.location ?? ''));
  let color = $state(untrack(() => event?.color ?? '#4A90D9'));
  let saving = $state(false);
  let confirmDelete = $state(false);

  const EVENT_COLORS = [
    '#4A90D9', '#E74C3C', '#2ECC71', '#F39C12',
    '#9B59B6', '#1ABC9C', '#E67E22', '#34495E',
  ];

  function toTs(date: string, time: string): number {
    return new Date(`${date}T${time}:00`).getTime();
  }

  async function handleSave() {
    if (!title.trim()) return;
    saving = true;

    const startTs = toTs(startDate, startTime);
    const endTs = toTs(startDate, endTime);

    if (isEditing && event) {
      const dto: UpdateEventDto = {
        id: event.id,
        title: title.trim(),
        description: description.trim() || undefined,
        start_ts: startTs,
        end_ts: endTs,
        all_day: allDay,
        location: location.trim() || undefined,
        color,
      };
      await updateEvent(dto);
    } else {
      const dto: CreateEventDto = {
        title: title.trim(),
        description: description.trim() || undefined,
        start_ts: startTs,
        end_ts: endTs,
        all_day: allDay,
        location: location.trim() || undefined,
        color,
      };
      await createEvent(dto);
    }

    saving = false;
    onClose();
  }

  async function handleDelete() {
    if (!event) return;
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    await deleteEvent(event.id);
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    if (e.key === 'Enter' && e.ctrlKey) handleSave();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Event" onkeydown={handleKeydown}>
  <div class="modal">
    <header class="modal-header">
      <h2 class="modal-title">{isEditing ? 'Edit event' : 'New event'}</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
    </header>

    <form class="modal-body" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
      <label class="field">
        <span class="field-label">Title *</span>
        <input
          type="text"
          class="field-input"
          bind:value={title}
          placeholder="Event title"
          required
          autofocus
        />
      </label>

      <div class="field-row">
        <label class="field">
          <span class="field-label">Date</span>
          <input type="date" class="field-input" bind:value={startDate} />
        </label>
        <label class="field checkbox-field">
          <input type="checkbox" bind:checked={allDay} />
          <span class="field-label">All day</span>
        </label>
      </div>

      {#if !allDay}
        <div class="field-row">
          <label class="field">
            <span class="field-label">Start</span>
            <input type="time" class="field-input" bind:value={startTime} />
          </label>
          <label class="field">
            <span class="field-label">End</span>
            <input type="time" class="field-input" bind:value={endTime} />
          </label>
        </div>
      {/if}

      <label class="field">
        <span class="field-label">Location</span>
        <input
          type="text"
          class="field-input"
          bind:value={location}
          placeholder="Optional location"
        />
      </label>

      <label class="field">
        <span class="field-label">Description</span>
        <textarea class="field-input" bind:value={description} rows="3" placeholder="Optional notes"></textarea>
      </label>

      <div class="field">
        <span class="field-label">Color</span>
        <div class="color-swatches">
          {#each EVENT_COLORS as c}
            <button
              type="button"
              class="swatch"
              class:selected={color === c}
              style:background-color={c}
              onclick={() => color = c}
              aria-label="Color {c}"
            ></button>
          {/each}
        </div>
      </div>
    </form>

    <footer class="modal-footer">
      {#if isEditing}
        <button
          type="button"
          class="btn btn-danger"
          onclick={handleDelete}
        >{confirmDelete ? 'Confirm delete' : 'Delete'}</button>
      {/if}
      <div class="footer-right">
        <button type="button" class="btn btn-ghost" onclick={onClose}>Cancel</button>
        <button
          type="button"
          class="btn btn-primary"
          onclick={handleSave}
          disabled={!title.trim() || saving}
        >{saving ? 'Saving…' : 'Save'}</button>
      </div>
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    background-color: var(--color-surface-elevated);
    border-radius: var(--radius-lg);
    width: 440px;
    max-width: 95vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .modal-title {
    font-size: var(--text-lg);
    font-weight: 600;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--text-base);
    color: var(--color-text-muted);
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

  .modal-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field-row {
    display: flex;
    gap: var(--space-3);
  }

  .field-row .field {
    flex: 1;
  }

  .checkbox-field {
    flex-direction: row !important;
    align-items: center;
    flex: 0 !important;
    white-space: nowrap;
  }

  .field-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .field-input {
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background-color: var(--color-surface);
    color: var(--color-text);
    font-size: var(--text-sm);
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }

  .field-input:focus {
    border-color: var(--color-accent);
  }

  textarea.field-input {
    resize: vertical;
    font-family: inherit;
    line-height: 1.5;
  }

  .color-swatches {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .swatch {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
    transition: transform 0.1s ease;
  }

  .swatch.selected {
    border-color: var(--color-text);
    transform: scale(1.2);
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--color-border);
    gap: var(--space-2);
  }

  .footer-right {
    display: flex;
    gap: var(--space-2);
    margin-left: auto;
  }

  .btn {
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: var(--color-accent);
    color: var(--color-on-accent);
  }

  .btn-ghost {
    background-color: transparent;
    color: var(--color-text);
    border: 1px solid var(--color-border);
  }

  .btn-danger {
    background-color: var(--color-error);
    color: white;
  }
</style>
