<script lang="ts">
  /**
   * HexTime View - time tracking
   *
   * HexTime runs as a sidecar server (see src-tauri/src/hextime.rs); this view
   * starts it on demand and shows the UI it serves.
   */

  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let url: string | null = null;
  let error: string | null = null;
  let isStarting = false;

  async function start() {
    isStarting = true;
    error = null;
    try {
      url = await invoke<string>("hextime_start");
    } catch (e) {
      error = String(e);
    } finally {
      isStarting = false;
    }
  }

  onMount(start);
</script>

<div class="hextime-view">
  {#if url}
    <iframe src={url} title="HexTime"></iframe>
  {:else if error}
    <div class="status">
      <p class="status-title">HexTime could not be started</p>
      <p class="status-detail">{error}</p>
      <button class="retry-button" on:click={start} disabled={isStarting}>Retry</button>
    </div>
  {:else}
    <div class="status">
      <div class="spinner" aria-hidden="true"></div>
      <p class="status-detail">Starting HexTime…</p>
    </div>
  {/if}
</div>

<style>
  .hextime-view {
    height: 100%;
    background: var(--bg-primary);
  }

  iframe {
    display: block;
    width: 100%;
    height: 100%;
    border: none;
  }

  .status {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 2rem;
    text-align: center;
  }

  .status-title {
    font-weight: 600;
    color: var(--text-primary);
  }

  .status-detail {
    max-width: 32rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    word-break: break-word;
  }

  .retry-button {
    padding: 0.5rem 1.25rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 6px;
    font-weight: 500;
  }

  .retry-button:disabled {
    opacity: 0.5;
  }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
