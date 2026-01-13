<script lang="ts">
  /**
   * Main Page - HexStickyNote Entry Point
   *
   * Contains the HUD. Orb is in a separate window.
   * Communicates with Orb window via Tauri events.
   */

  import { onMount } from 'svelte';
  import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import Hud from '$lib/components/Hud.svelte';

  let isHudOpen = false;

  onMount(() => {
    const window = getCurrentWindow();
    let unlistenClose: (() => void) | undefined;
    let unlistenMove: (() => void) | undefined;
    let unlistenOrb: (() => void) | undefined;

    // Initialize async operations
    (async () => {
      // Load saved position
      try {
        const state = await invoke<any>('load_window_state');
        if (state.main_window) {
          await window.setPosition(new PhysicalPosition(state.main_window.x, state.main_window.y));
        }
      } catch (e) {
        console.error('Failed to load main window position:', e);
      }

      // Handle window close - exit entire application
      unlistenClose = await window.onCloseRequested(async () => {
        await invoke('exit_app');
      });

      // Save position when window is moved
      unlistenMove = await window.onMoved(async ({ payload }) => {
        try {
          await invoke('save_main_window_position', {
            x: payload.x,
            y: payload.y
          });
        } catch (e) {
          console.error('Failed to save main window position:', e);
        }
      });

      // Listen for Orb clicks from orb window
      unlistenOrb = await listen('orb-clicked', () => {
        toggleHud();
      });
    })();

    return () => {
      unlistenClose?.();
      unlistenMove?.();
      unlistenOrb?.();
    };
  });

  async function toggleHud() {
    isHudOpen = !isHudOpen;

    // Show or hide window based on HUD state
    const window = getCurrentWindow();
    if (isHudOpen) {
      await window.show();
      await window.setFocus();
    } else {
      await window.hide();
    }

    // Notify orb window of state change
    await emit('hud-state-changed', { isOpen: isHudOpen });
  }

  // Handle global keyboard shortcuts
  async function handleKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd + K to toggle HUD
    if ((event.ctrlKey || event.metaKey) && event.key === 'k') {
      event.preventDefault();
      toggleHud();
    }

    // Escape to close HUD (only if not editing)
    if (event.key === 'Escape' && isHudOpen) {
      // Don't close if we're in an input field
      const activeElement = document.activeElement;
      if (activeElement?.tagName === 'INPUT' || activeElement?.tagName === 'TEXTAREA') {
        return;
      }
      toggleHud();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="app">
  <Hud isOpen={isHudOpen} />
</main>

<style>
  .app {
    height: 100vh;
    width: 100vw;
    position: relative;
    background: transparent;
    overflow: hidden;
  }
</style>
