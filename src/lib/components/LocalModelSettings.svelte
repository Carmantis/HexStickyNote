<script lang="ts">
  /**
   * Local Model Settings
   *
   * - Pick the model used for AI writing from models downloaded by the app
   *   and models installed in a local Ollama
   * - Download new models from the Ollama library
   */

  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { settingsStore, formatBytes, type LocalModel } from '$lib/stores/settingsStore';

  interface DownloadProgress {
    name: string;
    bytes_downloaded: number;
    total_bytes: number | null;
    percentage: number;
  }

  let modelName = '';
  let isDownloading = false;
  let downloadingName = '';
  let downloadProgress: DownloadProgress | null = null;
  let downloadError: string | null = null;
  let actionError: string | null = null;

  let unlistenProgress: UnlistenFn | undefined;

  $: models = $settingsStore.models;
  $: appModels = models.filter(m => m.source === 'app');
  $: ollamaModels = models.filter(m => m.source === 'ollama');
  $: activeModelId = $settingsStore.activeModelId;
  $: activeModel = models.find(m => m.id === activeModelId) ?? null;
  $: isRefreshing = $settingsStore.isLoading;

  onMount(async () => {
    await settingsStore.loadModels();

    unlistenProgress = await listen<DownloadProgress>('local-model-download-progress', (event) => {
      downloadProgress = event.payload;
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
  });

  function describe(model: LocalModel): string {
    const extra = [model.details, formatBytes(model.size)].filter(Boolean).join(' · ');
    return extra ? `${model.name} (${extra})` : model.name;
  }

  async function handleSelect(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    actionError = null;
    await settingsStore.setActiveModel(value || null);
  }

  async function handleDownload() {
    const name = modelName.trim();
    if (!name || isDownloading) return;

    isDownloading = true;
    downloadingName = name;
    downloadProgress = null;
    downloadError = null;

    try {
      const id = await invoke<string>('download_model', { name });
      modelName = '';
      await settingsStore.loadModels();
      // Use the first downloaded model straight away
      if (!$settingsStore.activeModelId) {
        await settingsStore.setActiveModel(id);
      }
    } catch (e) {
      const message = String(e);
      downloadError = message === 'Download cancelled' ? null : message;
    } finally {
      isDownloading = false;
      downloadProgress = null;
    }
  }

  async function handleCancel() {
    await invoke('cancel_model_download');
  }

  async function handleDelete() {
    if (!activeModel || activeModel.source !== 'app') return;
    if (!confirm(`Delete the downloaded model ${activeModel.name}?`)) return;

    try {
      await invoke('delete_local_model', { modelId: activeModel.id });
      actionError = null;
    } catch (e) {
      actionError = String(e);
    }
    await settingsStore.loadModels();
  }
</script>

<div class="local-model-settings">
  <!-- Model in use -->
  <div class="form-group">
    <label for="model-select" class="input-label">Model in use</label>
    <div class="select-row">
      <select
        id="model-select"
        class="styled-select"
        value={activeModelId ?? ''}
        on:change={handleSelect}
        disabled={models.length === 0 && !activeModelId}
      >
        {#if models.length === 0}
          <option value="">No models yet – add one below</option>
        {:else}
          <option value="">Select a model…</option>
        {/if}
        {#if activeModelId && !activeModel}
          <option value={activeModelId} disabled>
            {activeModelId.replace(/^(app|ollama):/, '')} (unavailable)
          </option>
        {/if}
        {#if appModels.length > 0}
          <optgroup label="Downloaded">
            {#each appModels as model (model.id)}
              <option value={model.id}>{describe(model)}</option>
            {/each}
          </optgroup>
        {/if}
        {#if ollamaModels.length > 0}
          <optgroup label="Ollama">
            {#each ollamaModels as model (model.id)}
              <option value={model.id}>{describe(model)}</option>
            {/each}
          </optgroup>
        {/if}
      </select>

      <button
        class="icon-button"
        on:click={() => settingsStore.loadModels()}
        disabled={isRefreshing}
        title="Refresh model list"
        aria-label="Refresh model list"
      >
        <svg class:spinning={isRefreshing} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 12a9 9 0 1 1-2.64-6.36"/>
          <polyline points="21 3 21 9 15 9"/>
        </svg>
      </button>
    </div>

    <p class="config-hint">
      {#if $settingsStore.ollamaAvailable}
        Ollama models run through Ollama; downloaded models run inside HexStickyNote.
      {:else}
        Ollama is not running – start it to use the models you have installed there.
      {/if}
    </p>

    {#if activeModelId && !activeModel}
      <p class="warning-text">The selected model is not available right now.</p>
    {/if}

    {#if activeModel?.source === 'app'}
      <button class="delete-model-button" on:click={handleDelete}>
        Delete downloaded model
      </button>
    {/if}

    {#if actionError}
      <div class="error-message">{actionError}</div>
    {/if}
  </div>

  <!-- Add a model -->
  <div class="form-group">
    <label for="model-name-input" class="input-label">Add model from the Ollama library</label>

    {#if isDownloading}
      <div class="download-progress">
        <div class="progress-info">
          <span>Downloading {downloadingName}…</span>
          {#if downloadProgress}
            <span class="progress-percentage">{downloadProgress.percentage.toFixed(1)}%</span>
          {/if}
        </div>
        <div class="progress-bar">
          <div class="progress-fill" style="width: {downloadProgress?.percentage || 0}%"></div>
        </div>
        <div class="progress-footer">
          <span class="progress-details">
            {#if downloadProgress?.total_bytes}
              {formatBytes(downloadProgress.bytes_downloaded)} / {formatBytes(downloadProgress.total_bytes)}
            {:else}
              Fetching model information…
            {/if}
          </span>
          <button class="cancel-button" on:click={handleCancel}>Cancel</button>
        </div>
      </div>
    {:else}
      <div class="select-row">
        <input
          id="model-name-input"
          type="text"
          class="styled-input"
          bind:value={modelName}
          placeholder="e.g. llama3.2:3b"
          spellcheck="false"
          autocomplete="off"
          on:keydown={(e) => e.key === 'Enter' && handleDownload()}
        />
        <button class="download-button" on:click={handleDownload} disabled={!modelName.trim()}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          Download
        </button>
      </div>
      <p class="config-hint">
        Use a name from ollama.com/library, e.g. <code>qwen2.5:7b</code>, or a Hugging Face GGUF
        repository such as <code>hf.co/user/repo:Q4_K_M</code>.
      </p>
    {/if}

    {#if downloadError}
      <div class="error-message">{downloadError}</div>
    {/if}
  </div>
</div>

<style>
  .local-model-settings {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .select-row {
    display: flex;
    gap: 0.5rem;
    align-items: stretch;
  }

  .styled-select,
  .styled-input {
    flex: 1;
    min-width: 0;
    padding: 0.75rem 1rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 0.875rem;
    transition: border-color var(--transition-fast);
  }

  .styled-select:hover:not(:disabled),
  .styled-input:hover {
    border-color: var(--accent-primary);
  }

  .styled-select:focus,
  .styled-input:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .styled-select option {
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .styled-input::placeholder {
    color: var(--text-muted);
  }

  .icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.875rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-secondary);
    transition: border-color var(--transition-fast), color var(--transition-fast);
  }

  .icon-button:hover:not(:disabled) {
    border-color: var(--accent-primary);
    color: var(--text-primary);
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .config-hint {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .config-hint code {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .warning-text {
    margin: 0;
    font-size: 0.75rem;
    color: #f59e0b;
  }

  .error-message {
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #ef4444;
    font-size: 0.875rem;
    word-break: break-word;
  }

  .download-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0 1rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 6px;
    font-weight: 500;
    transition: background var(--transition-fast), opacity var(--transition-fast);
  }

  .download-button:hover:not(:disabled) {
    background: var(--accent-secondary);
  }

  .download-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .delete-model-button,
  .cancel-button {
    align-self: flex-start;
    padding: 0.5rem 1rem;
    background: transparent;
    color: #ef4444;
    border: 1px solid #ef4444;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background var(--transition-fast);
  }

  .delete-model-button:hover,
  .cancel-button:hover {
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
    gap: 0.5rem;
    font-size: 0.875rem;
    word-break: break-all;
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

  .progress-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .progress-details {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
