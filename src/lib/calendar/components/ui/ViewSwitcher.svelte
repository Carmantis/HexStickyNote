<svelte:options runes={true} />

<script lang="ts">
  import type { ViewMode } from '$lib/calendar/types';

  interface Props {
    active: ViewMode;
    onSwitch: (mode: ViewMode) => void;
  }

  let { active, onSwitch }: Props = $props();

  const modes: { value: ViewMode; label: string }[] = [
    { value: 'month', label: 'Month' },
    { value: 'week', label: 'Week' },
    { value: 'day', label: 'Day' },
  ];
</script>

<div class="view-switcher" role="tablist" aria-label="Calendar view">
  {#each modes as mode}
    <button
      role="tab"
      aria-selected={active === mode.value}
      class="switch-btn"
      class:active={active === mode.value}
      onclick={() => onSwitch(mode.value)}
    >{mode.label}</button>
  {/each}
</div>

<style>
  .view-switcher {
    display: flex;
    background-color: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }

  .switch-btn {
    padding: 4px 14px;
    border-radius: calc(var(--radius-md) - 2px);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    background: none;
    border: none;
    color: var(--color-text-muted);
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .switch-btn:hover:not(.active) {
    background-color: var(--color-surface-hover);
    color: var(--color-text);
  }

  .switch-btn.active {
    background-color: var(--color-accent);
    color: var(--color-on-accent);
    font-weight: 600;
  }
</style>
