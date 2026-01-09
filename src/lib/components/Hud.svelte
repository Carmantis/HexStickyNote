<script lang="ts">
  /**
   * HUD Component - Main workspace view
   */

  import { onMount } from "svelte";
  // Poistettu tarpeeton getCurrentWindow importti
  import { cardStore } from "$lib/stores/cardStore";
  import { settingsStore } from "$lib/stores/settingsStore";
  import CardCarousel from "./CardCarousel.svelte";
  import Settings from "./Settings.svelte";

  export let isOpen: boolean = false;

  let showSettings = false;

  onMount(async () => {
    await Promise.all([cardStore.loadCards(), settingsStore.loadProviders()]);
  });

  function handleSettingsClick() {
    showSettings = true;
  }

  function handleSettingsClose() {
    showSettings = false;
  }

  // Poistettu manuaalinen raahausfunktio (handleHeaderMouseDown)
</script>

{#if isOpen}
  <div class="hud" class:visible={isOpen}>
    <header class="hud-header" data-tauri-drag-region>
      <h1 class="hud-title" data-tauri-drag-region>
        <span class="title-icon" data-tauri-drag-region>
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polygon
              points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
            />
          </svg>
        </span>
        HexStickyNote
      </h1>

      <div class="hud-actions">
        <button
          class="settings-button"
          on:click={handleSettingsClick}
          title="Settings"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        </button>
      </div>
    </header>

    <main class="hud-content">
      <CardCarousel />
    </main>

    {#if showSettings}
      <Settings on:close={handleSettingsClose} />
    {/if}
  </div>
{/if}

<style>
  .hud {
    position: fixed;
    inset: 0;
    background: transparent;
    z-index: 100;
    display: flex;
    flex-direction: column;
    opacity: 0;
    transform: scale(0.98);
    transition:
      opacity var(--transition-normal),
      transform var(--transition-normal);
    overflow: hidden;
    border-radius: 12px;
    /* Estetään klikkaukset tyhjään tilaan, mutta sallitaan header ja sisältö */
    pointer-events: none;
  }

  .hud.visible {
    opacity: 1;
    transform: scale(1);
  }

  .hud-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    background: var(--bg-secondary);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    cursor: default; /* Normaali kursori */
    user-select: none;
    /* Sallitaan klikkaukset headeriin */
    pointer-events: auto;
  }

  /* Kun hiiri on headerin päällä, näytetään move-kursori, 
     paitsi jos ollaan napin päällä */
  .hud-header:hover {
    cursor: move;
  }

  .hud-title {
    font-size: 1.25rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
  }

  .title-icon {
    color: var(--accent-primary);
    display: flex;
    align-items: center;
  }

  .hud-actions {
    display: flex;
    gap: 0.5rem;
  }

  .settings-button {
    background: transparent;
    color: var(--text-secondary);
    padding: 0.5rem;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      background var(--transition-fast),
      color var(--transition-fast);
    /* Tärkeä: osoittaa käyttäjälle että tätä klikataan, ei raahata */
    cursor: pointer;
    pointer-events: auto;
  }

  .settings-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .hud-content {
    flex: 1;
    overflow: hidden;
    pointer-events: auto; /* Sisältö pitää olla klikattavissa */
  }
</style>
