<script lang="ts">
  /**
   * Orb Window Page - Separate floating Orb window
   *
   * This window stays always on top and is always interactive.
   * Communicates with main window via Tauri events.
   */

  import { getCurrentWindow, PhysicalPosition, primaryMonitor } from '@tauri-apps/api/window';
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import Orb from '$lib/components/Orb.svelte';

  let isHudOpen = false;

  onMount(() => {
    const window = getCurrentWindow();
    let unlistenMove: (() => void) | undefined;
    let unlistenHud: (() => void) | undefined;

    // Initialize async operations
    (async () => {
      // Load saved position or use default
      try {
        const state = await invoke<any>('load_window_state');
        if (state.orb_window) {
          await window.setPosition(new PhysicalPosition(state.orb_window.x, state.orb_window.y));
        } else {
          // Default: position orb in bottom-right corner of primary monitor
          const monitor = await primaryMonitor();
          if (monitor) {
            const x = monitor.size.width - 110;
            const y = monitor.size.height - 110;
            await window.setPosition(new PhysicalPosition(x, y));
          }
        }
      } catch (e) {
        console.error('Failed to load or set orb window position:', e);
      }

      // Save position when window is moved
      unlistenMove = await window.onMoved(async ({ payload }) => {
        try {
          await invoke('save_orb_window_position', {
            x: payload.x,
            y: payload.y
          });
        } catch (e) {
          console.error('Failed to save orb position:', e);
        }
      });

      // Listen for HUD state changes from main window
      unlistenHud = await listen('hud-state-changed', (event: any) => {
        isHudOpen = event.payload.isOpen;
      });
    })();

    return () => {
      unlistenMove?.();
      unlistenHud?.();
    };
  });

  async function handleOrbToggle() {
    // Send toggle event to main window
    await emit('orb-clicked');
  }
</script>

<main class="orb-window">
  <Orb {isHudOpen} on:toggle={handleOrbToggle} />
</main>

<style>
  .orb-window {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
  }
</style>
