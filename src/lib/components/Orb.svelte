<script lang="ts">
  /**
   * Orb Component - Floating launcher button
   *
   * The primary entry point to the HUD.
   * Floats in the corner of the screen.
   */

  import { createEventDispatcher } from 'svelte';

  export let isHudOpen: boolean = false;

  const dispatch = createEventDispatcher<{ toggle: void }>();

  function handleClick() {
    dispatch('toggle');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleClick();
    }
  }
</script>

<button
  class="orb"
  class:active={isHudOpen}
  on:click={handleClick}
  on:keydown={handleKeydown}
  title={isHudOpen ? 'Close HUD' : 'Open HUD'}
  aria-label={isHudOpen ? 'Close HUD' : 'Open HUD'}
  aria-expanded={isHudOpen}
>
  <div class="orb-inner">
    <div class="orb-icon">
      {#if isHudOpen}
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6 6 18"/>
          <path d="m6 6 12 12"/>
        </svg>
      {:else}
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>
        </svg>
      {/if}
    </div>
  </div>
  <div class="orb-glow"></div>
</button>

<style>
  .orb {
    position: fixed;
    bottom: 2rem;
    right: 2rem;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-primary), var(--accent-secondary));
    border: none;
    cursor: pointer;
    z-index: 10000;
    padding: 0;
    transition: transform var(--transition-fast);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }

  .orb:hover {
    transform: scale(1.1);
  }

  .orb:active {
    transform: scale(0.95);
  }

  .orb.active {
    background: linear-gradient(135deg, #ef4444, #f87171);
  }

  .orb-inner {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2;
  }

  .orb-icon {
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .orb-glow {
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    background: var(--accent-primary);
    filter: blur(12px);
    opacity: 0.4;
    z-index: 1;
    transition: opacity var(--transition-fast);
  }

  .orb:hover .orb-glow {
    opacity: 0.6;
  }

  .orb.active .orb-glow {
    background: #ef4444;
  }
</style>
