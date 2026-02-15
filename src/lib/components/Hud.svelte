<script lang="ts">
  /**
   * HUD Component - Main workspace view
   */

  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { cardStore, editingCard } from "$lib/stores/cardStore";
  import { settingsStore } from "$lib/stores/settingsStore";
  import CardCarousel from "./CardCarousel.svelte";
  import Settings from "./Settings.svelte";

  export let isOpen: boolean = false;

  let showSettings = false;
  let isGhostMode = false;
  $: editing = $editingCard;

  onMount(async () => {
    await Promise.all([cardStore.loadCards(), settingsStore.loadProviders()]);
  });

  function startResize(direction: string) {
    getCurrentWindow().startResizeDragging(direction as any);
  }

  async function handleAddCard() {
    const card = await cardStore.createCard('# New Note\n\nStart typing...');
    if (card) {
      cardStore.enterEditMode(card.id);
    }
  }

  async function handleDeleteCard() {
    if (editing) {
      if (confirm('Are you sure you want to delete this note?')) {
        await cardStore.deleteCard(editing.id);
      }
    }
  }

  function handleSettingsClick() {
    showSettings = true;
  }

  function handleSettingsClose() {
    showSettings = false;
  }

  function handleHeaderMouseDown(event: MouseEvent) {
    // Only drag on left click and if not clicking a button
    if (event.button === 0 && !(event.target as Element).closest('button')) {
      getCurrentWindow().startDragging();
    }
  }

  async function handleMinimize() {
    await getCurrentWindow().minimize();
  }

  async function handleClose() {
    // Exit entire application (all windows)
    await invoke('exit_app');
  }

  async function toggleGhostMode() {
    isGhostMode = !isGhostMode;
    const window = getCurrentWindow();

    if (isGhostMode) {
      // Enable click-through and make more transparent
      await window.setIgnoreCursorEvents(true);
    } else {
      // Disable click-through
      await window.setIgnoreCursorEvents(false);
    }
  }

  async function handleOpenCardsFolder() {
    try {
      await invoke('open_cards_directory');
    } catch (e) {
      console.error('Failed to open cards directory:', e);
    }
  }

  async function handleRefreshCards() {
    await cardStore.loadCards();
  }
</script>

{#if isOpen}
  <div class="hud" class:visible={isOpen}>
    <!-- Resize Handles (Invisible window borders) -->
    <div class="resize-edge top" on:mousedown={() => startResize('North')}></div>
    <div class="resize-edge bottom" on:mousedown={() => startResize('South')}></div>
    <div class="resize-edge left" on:mousedown={() => startResize('West')}></div>
    <div class="resize-edge right" on:mousedown={() => startResize('East')}></div>
    <div class="resize-corner top-left" on:mousedown={() => startResize('NorthWest')}></div>
    <div class="resize-corner top-right" on:mousedown={() => startResize('NorthEast')}></div>
    <div class="resize-corner bottom-left" on:mousedown={() => startResize('SouthWest')}></div>
    <div class="resize-corner bottom-right" on:mousedown={() => startResize('SouthEast')}></div>

    <header class="hud-header" on:mousedown={handleHeaderMouseDown}>
      <div class="hud-left-actions">
        <button
          class="action-button"
          on:click|stopPropagation={handleOpenCardsFolder}
          title="Open Cards Folder"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </button>

        <button
          class="action-button"
          on:click|stopPropagation={handleRefreshCards}
          title="Refresh Cards"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="23 4 23 10 17 10" />
            <polyline points="1 20 1 14 7 14" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
        </button>
      </div>

      <h1 class="hud-title">
        <span class="title-icon">
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
        <!-- Action Buttons (Note mgmt) -->
        {#if editing}
          <button
            class="action-button delete-button"
            on:click|stopPropagation={handleDeleteCard}
            title="Delete Note"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
          </button>
        {:else}
          <button
            class="action-button"
            on:click|stopPropagation={handleAddCard}
            title="New Note"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14" />
              <path d="M5 12h14" />
            </svg>
          </button>
        {/if}

        <button
          class="action-button"
          on:click|stopPropagation={handleSettingsClick}
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

        <button
          class="action-button close-button"
          on:click|stopPropagation={handleClose}
          title="Close Application"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 6L6 18" />
            <path d="M6 6l12 12" />
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
    padding: 0.75rem 1.5rem;
    width: 600px;
    margin: 1.5rem auto 0;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--border-radius);
    background: var(--bg-secondary);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    cursor: default; /* Normaali kursori */
    user-select: none;
    /* Sallitaan klikkaukset headeriin */
    pointer-events: auto;
    position: relative;
    min-height: 60px;
    box-shadow: var(--shadow-md);
  }

  /* Resize Handles (Invisible) */
  .resize-edge {
    position: absolute;
    z-index: 1000;
    pointer-events: auto; /* Catch clicks even if parent is none */
  }
  .resize-corner {
    position: absolute;
    width: 10px;
    height: 10px;
    z-index: 1001;
    pointer-events: auto;
  }

  .resize-edge.top { top: 0; left: 0; right: 0; height: 5px; cursor: n-resize; }
  .resize-edge.bottom { bottom: 0; left: 0; right: 0; height: 5px; cursor: s-resize; }
  .resize-edge.left { top: 0; bottom: 0; left: 0; width: 5px; cursor: w-resize; }
  .resize-edge.right { top: 0; bottom: 0; right: 0; width: 5px; cursor: e-resize; }

  .resize-corner.top-left { top: 0; left: 0; cursor: nw-resize; }
  .resize-corner.top-right { top: 0; right: 0; cursor: ne-resize; }
  .resize-corner.bottom-left { bottom: 0; left: 0; cursor: sw-resize; }
  .resize-corner.bottom-right { bottom: 0; right: 0; cursor: se-resize; }

  /* Kun hiiri on headerin päällä, näytetään move-kursori, 
     paitsi jos ollaan napin päällä */
  .hud-header:hover {
    cursor: move;
  }

  .hud-left-actions {
    display: flex;
    gap: 0.5rem;
  }

  .hud-title {
    font-size: 1.1rem;
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
  }

  .title-icon {
    color: var(--accent-primary);
    display: flex;
    align-items: center;
  }

  .hud-actions {
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
    z-index: 2;
  }

  .settings-button,
  .action-button {
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

  .settings-button:hover,
  .action-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .delete-button:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .close-button:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .hud-content {
    flex: 1;
    overflow: hidden;
    pointer-events: auto; /* Sisältö pitää olla klikattavissa */
  }
</style>
