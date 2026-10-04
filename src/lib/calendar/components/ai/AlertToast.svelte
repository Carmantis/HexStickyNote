<svelte:options runes={true} />

<script lang="ts">
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { sendNotification } from '@tauri-apps/plugin-notification';
  import { toast, showToast, dismissToast } from '$lib/calendar/stores/ai.svelte';
  import { onDestroy } from 'svelte';

  let unlisten: UnlistenFn | undefined;

  // Listen for reminder_due events emitted by the Rust scheduler
  listen<{ title: string; body: string; event_id: string }>(
    'reminder_due',
    (event) => {
      showToast(event.payload.title, event.payload.body);
      // Also fire OS notification
      try {
        sendNotification({ title: event.payload.title, body: event.payload.body });
      } catch {
        // Notifications unavailable; the in-app toast is still shown
      }
    }
  ).then((fn) => { unlisten = fn; });

  onDestroy(() => { unlisten?.(); });

  let autoHideTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (toast.active) {
      if (autoHideTimer) clearTimeout(autoHideTimer);
      autoHideTimer = setTimeout(dismissToast, 8000);
    }
  });
</script>

{#if toast.active}
  <div class="toast" role="alert" aria-live="polite">
    <div class="toast-body">
      <strong class="toast-title">{toast.active.title}</strong>
      <span class="toast-msg">{toast.active.body}</span>
    </div>
    <button class="toast-close" onclick={dismissToast} aria-label="Dismiss">✕</button>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: var(--space-4);
    right: var(--space-4);
    z-index: 9999;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    background-color: var(--color-surface-elevated);
    border: 1px solid var(--color-border);
    border-left: 4px solid var(--color-accent);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
    max-width: 360px;
    animation: slide-in 0.25s ease;
  }

  @keyframes slide-in {
    from { transform: translateX(120%); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  .toast-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .toast-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-text);
  }

  .toast-msg {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }

  .toast-close {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: var(--text-base);
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

  .toast-close:hover {
    background-color: var(--color-surface-hover);
  }
</style>
