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

  let customUrl = '';
  let repoName = '';
  let filename = '';
  let showAdvanced = false;

  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;

  onMount(async () => {
    await loadModelStatus();

    // Load custom configuration if available
    try {
      const settings = await invoke<{
        local_models: Record<string, { custom_url?: string; repo?: string; filename?: string }>
      }>('get_all_settings');
      const localConfig = settings.local_models?.[provider.id];
      if (localConfig) {
        customUrl = localConfig.custom_url || '';
        repoName = localConfig.repo || '';
        filename = localConfig.filename || '';
      }
    } catch (e) {
      console.error('Failed to load local model config:', e);
    }

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

  async function handleSaveConfig() {
    try {
      await invoke('set_local_model_config', {
        provider: provider.id,
        repo: repoName || '',
        filename: filename || '',
        custom_url: customUrl || null
      });
      // Show success or reload status
      await loadModelStatus();
    } catch (e) {
      console.error('Failed to save config:', e);
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
      <p>Model not downloaded. Configure and download to use offline.</p>

      <button
        class="toggle-advanced"
        on:click={() => showAdvanced = !showAdvanced}
      >
        {showAdvanced ? '▼' : '▶'} Advanced Configuration
      </button>

      {#if showAdvanced}
        <div class="advanced-config">
          <div class="config-section">
            <label class="config-label">
              Custom Download URL (optional)
              <input
                type="text"
                bind:value={customUrl}
                placeholder="https://huggingface.co/user/repo/resolve/main/model.gguf"
                class="config-input"
              />
            </label>
            <p class="config-hint">Direct link to .gguf file. Overrides repo/filename if set.</p>
          </div>

          <div class="or-divider">
            <span>OR use HuggingFace repo</span>
          </div>

          <div class="config-section">
            <label class="config-label">
              Repository
              <input
                type="text"
                bind:value={repoName}
                placeholder="mradermacher/Llama-Poro-2-8B-Instruct-GGUF"
                class="config-input"
              />
            </label>

            <label class="config-label">
              Filename
              <input
                type="text"
                bind:value={filename}
                placeholder="Llama-Poro-2-8B-Instruct.Q4_K_M.gguf"
                class="config-input"
              />
            </label>
            <p class="config-hint">Leave empty to use defaults</p>
          </div>

          <button class="save-config-button" on:click={handleSaveConfig}>
            Save Configuration
          </button>
        </div>
      {/if}

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

  .toggle-advanced {
    padding: 0.5rem;
    background: transparent;
    color: var(--text-secondary);
    border: none;
    font-size: 0.875rem;
    cursor: pointer;
    transition: color var(--transition-fast);
    text-align: left;
    width: 100%;
  }

  .toggle-advanced:hover {
    color: var(--text-primary);
  }

  .advanced-config {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
  }

  .config-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .config-label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .config-input {
    padding: 0.5rem 0.75rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-family: 'Courier New', monospace;
  }

  .config-input:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
  }

  .config-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0;
    font-style: italic;
  }

  .or-divider {
    text-align: center;
    position: relative;
    margin: 0.5rem 0;
  }

  .or-divider span {
    display: inline-block;
    padding: 0 0.5rem;
    background: var(--bg-card);
    color: var(--text-muted);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .or-divider::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 1px;
    background: rgba(255, 255, 255, 0.1);
    z-index: -1;
  }

  .save-config-button {
    padding: 0.5rem 1rem;
    background: rgba(99, 102, 241, 0.2);
    color: var(--accent-primary);
    border: 1px solid var(--accent-primary);
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background var(--transition-fast);
  }

  .save-config-button:hover {
    background: rgba(99, 102, 241, 0.3);
  }
</style>
