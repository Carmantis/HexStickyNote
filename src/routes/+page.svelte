<script lang="ts">
  /**
   * Main Page - HexStickyNote Entry Point
   *
   * Contains the Orb launcher and HUD.
   */

  import Orb from '$lib/components/Orb.svelte';
  import Hud from '$lib/components/Hud.svelte';

  let isHudOpen = false;

  function handleOrbToggle() {
    isHudOpen = !isHudOpen;
  }

  // Handle global keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd + K to toggle HUD
    if ((event.ctrlKey || event.metaKey) && event.key === 'k') {
      event.preventDefault();
      isHudOpen = !isHudOpen;
    }

    // Escape to close HUD (only if not editing)
    if (event.key === 'Escape' && isHudOpen) {
      // Don't close if we're in an input field
      const activeElement = document.activeElement;
      if (activeElement?.tagName === 'INPUT' || activeElement?.tagName === 'TEXTAREA') {
        return;
      }
      isHudOpen = false;
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="app">
  <Hud isOpen={isHudOpen} />
  <Orb {isHudOpen} on:toggle={handleOrbToggle} />
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
