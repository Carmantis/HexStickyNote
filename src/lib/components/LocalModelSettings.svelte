<script lang="ts">
  /**
   * Local Model Settings Component
   *
   * Handles downloading and managing local GGUF models
   */

  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  export let provider: { id: string; name: string; configured: boolean };

  interface ModelStatus {
    provider: string;
    is_downloaded: boolean;
    file_size: number | null;
    path: string | null;
  }

  interface DownloadProgress {
    provider: string;
    bytes_downloaded: number;
    total_bytes: number | null;
    percentage: number;
  }

  let modelStatus: ModelStatus | null = null;
  let isDownloading = false;
  let downloadProgress: DownloadProgress | null = null;
  let error: string | null = null;

  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;

  onMount(async () => {
    await loadModelStatus();

    // Listen for download progress
    unlistenProgress = await listen<DownloadProgress>('local-model-download-progress', (event) => {
      if (event.payload.provider === provider.id) {
        downloadProgress = event.payload;
      }
    });

    // Listen for download complete
    unlistenComplete = await listen<{ provider: string; path: string }>('local-model-download-complete', (event) => {
      if (event.payload.provider === provider.id) {
        isDownloading = false;
        downloadProgress = null;
        loadModelStatus();
      }
    });
  });

  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenComplete) unlistenComplete();
  });

  async function loadModelStatus() {
    try {
      modelStatus = await invoke<ModelStatus>('get_local_model_status', { provider: provider.id });
    } catch (e) {
      console.error('Failed to load model status:', e);
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleDownload() {
    error = null;
    isDownloading = true;
    downloadProgress = { provider: provider.id, bytes_downloaded: 0, total_bytes: null, percentage: 0 };

    try {
      await invoke('download_local_model', { provider: provider.id });
    } catch (e) {
      console.error('Failed to download model:', e);
      error = e instanceof Error ? e.message : String(e);
      isDownloading = false;
      downloadProgress = null;
    }
  }

  async function handleDelete() {
    if (!confirm(`Are you sure you want to delete the ${provider.name} model?`)) {
      return;
    }

    try {
      await invoke('delete_local_model', { provider: provider.id });
      await loadModelStatus();
    } catch (e) {
      console.error('Failed to delete model:', e);
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  }
</script>

<div class="local-model-settings">
  {#if error}
    <div class="error-message">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <path d="M12 8v4"/>
        <path d="M12 16h.01"/>
      </svg>
      {error}
    </div>
  {/if}

  {#if modelStatus?.is_downloaded}
    <div class="model-info">
      <div class="info-item">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
          <polyline points="22 4 12 14.01 9 11.01"/>
        </svg>
        <span>Model downloaded</span>
      </div>
      {#if modelStatus.file_size}
        <div class="info-item">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/>
            <polyline points="13 2 13 9 20 9"/>
          </svg>
          <span>{formatBytes(modelStatus.file_size)}</span>
        </div>
      {/if}
    </div>

    <button class="delete-model-button" on:click={handleDelete}>
      Delete Model
    </button>
  {:else if isDownloading}
    <div class="download-progress">
      <div class="progress-info">
        <span>Downloading...</span>
        {#if downloadProgress}
          <span class="progress-percentage">{downloadProgress.percentage.toFixed(1)}%</span>
        {/if}
      </div>
      <div class="progress-bar">
        <div
          class="progress-fill"
          style="width: {downloadProgress?.percentage || 0}%"
        ></div>
      </div>
      {#if downloadProgress && downloadProgress.total_bytes}
        <div class="progress-details">
          {formatBytes(downloadProgress.bytes_downloaded)} / {formatBytes(downloadProgress.total_bytes)}
        </div>
      {/if}
    </div>
  {:else}
    <div class="model-not-downloaded">
      <p>Model not downloaded. Download to use offline.</p>

      <button class="download-button" on:click={handleDownload}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        Download Model
      </button>
    </div>
  {/if}
</div>

<style>
  .local-model-settings {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .error-message {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #ef4444;
    font-size: 0.875rem;
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: 6px;
  }

  .info-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: #22c55e;
    font-size: 0.875rem;
  }

  .model-not-downloaded {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .model-not-downloaded p {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .download-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 6px;
    font-weight: 500;
    transition: background var(--transition-fast);
  }

  .download-button:hover {
    background: var(--accent-secondary);
  }

  .delete-model-button {
    padding: 0.5rem 1rem;
    background: transparent;
    color: #ef4444;
    border: 1px solid #ef4444;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background var(--transition-fast);
  }

  .delete-model-button:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .download-progress {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.875rem;
  }

  .progress-percentage {
    font-weight: 600;
    color: var(--accent-primary);
  }

  .progress-bar {
    height: 8px;
    background: var(--bg-secondary);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
    transition: width 0.3s ease;
  }

  .progress-details {
    font-size: 0.75rem;
    color: var(--text-muted);
    text-align: center;
  }
</style>
