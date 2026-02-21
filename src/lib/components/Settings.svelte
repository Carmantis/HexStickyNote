<script lang="ts">
  /**
   * Settings Component - AI Provider Configuration
   *
   * Allows users to:
   * - Select active AI provider
   * - Enter/update API keys
   * - View provider status
   */

  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settingsStore, activeProvider } from '$lib/stores/settingsStore';
  import LocalModelSettings from './LocalModelSettings.svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  // Local model provider IDs (no API key needed)
  const LOCAL_MODELS = ['poro2_8b', 'llama3_8b'];

  // GPU type state
  let gpuType = 'cpu';

  // Cloud provider IDs
  const CLOUD_PROVIDERS = ['openai', 'anthropic', 'google'];

  function isLocalModel(providerId: string): boolean {
    return LOCAL_MODELS.includes(providerId);
  }

  // Unified cloud provider state
  let selectedCloudProvider = 'openai';
  let apiKey = '';
  let showApiKey = false;

  // Model selection state
  let recommendedModels: Record<string, Array<{ id: string; name: string }>> = {};
  let selectedModel = '';
  let isCustomModel = false;
  let customModelInput = '';

  // Claude Desktop MCP state
  let claudeInstalled = false;
  let mcpConfigured = false;
  let mcpLoading = false;

  $: providers = $settingsStore.providers;
  $: activeProviderId = $settingsStore.activeProviderId;
  $: isLoading = $settingsStore.isLoading;

  async function checkClaudeMcp() {
    try {
      const status = await invoke<{ claude_installed: boolean; mcp_configured: boolean }>('check_claude_mcp');
      claudeInstalled = status.claude_installed;
      mcpConfigured = status.mcp_configured;
    } catch (e) {
      console.error('Failed to check Claude MCP status:', e);
    }
  }

  async function handleSetupMcp() {
    mcpLoading = true;
    try {
      await invoke('setup_claude_mcp');
      mcpConfigured = true;
    } catch (e) {
      console.error('Failed to setup Claude MCP:', e);
    }
    mcpLoading = false;
  }

  async function handleRemoveMcp() {
    mcpLoading = true;
    try {
      await invoke('remove_claude_mcp');
      mcpConfigured = false;
    } catch (e) {
      console.error('Failed to remove Claude MCP:', e);
    }
    mcpLoading = false;
  }

  onMount(async () => {
    // Fetch recommended models
    try {
      recommendedModels = await invoke<Record<string, Array<{ id: string; name: string }>>>('get_recommended_models');
    } catch (e) {
      console.error('Failed to fetch recommended models:', e);
    }

    // Fetch current settings to initialize selected model for cloud provider
    try {
      const settings = await invoke<{
        providers: Record<string, { model?: string; custom_model?: string }>,
        gpu_type: string
      }>('get_all_settings');

      // Initialize GPU type
      gpuType = settings.gpu_type || 'cpu';

      // Initialize from active provider or default to first cloud provider
      if (activeProviderId && CLOUD_PROVIDERS.includes(activeProviderId)) {
        selectedCloudProvider = activeProviderId;
      }

      const providerSettings = settings.providers[selectedCloudProvider];
      if (providerSettings) {
        if (providerSettings.custom_model) {
          isCustomModel = true;
          customModelInput = providerSettings.custom_model;
          selectedModel = 'custom';
        } else if (providerSettings.model) {
          selectedModel = providerSettings.model;
        } else if (recommendedModels[selectedCloudProvider]?.length > 0) {
          selectedModel = recommendedModels[selectedCloudProvider][0].id;
        }
      } else if (recommendedModels[selectedCloudProvider]?.length > 0) {
        selectedModel = recommendedModels[selectedCloudProvider][0].id;
      }
    } catch (e) {
      console.error('Failed to fetch settings:', e);
    }

    // Check Claude Desktop MCP status
    checkClaudeMcp();
  });

  function handleClose() {
    dispatch('close');
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  }

  async function handleCloudProviderChange(newProviderId: string) {
    selectedCloudProvider = newProviderId;
    apiKey = '';
    showApiKey = false;

    // Load model settings for new provider
    try {
      const settings = await invoke<{
        providers: Record<string, { model?: string; custom_model?: string }>
      }>('get_all_settings');

      const providerSettings = settings.providers[newProviderId];
      if (providerSettings) {
        if (providerSettings.custom_model) {
          isCustomModel = true;
          customModelInput = providerSettings.custom_model;
          selectedModel = 'custom';
        } else if (providerSettings.model) {
          isCustomModel = false;
          selectedModel = providerSettings.model;
        } else if (recommendedModels[newProviderId]?.length > 0) {
          isCustomModel = false;
          selectedModel = recommendedModels[newProviderId][0].id;
        }
      } else if (recommendedModels[newProviderId]?.length > 0) {
        isCustomModel = false;
        selectedModel = recommendedModels[newProviderId][0].id;
      }
    } catch (e) {
      console.error('Failed to fetch settings:', e);
    }
  }

  async function handleSaveCloudProvider() {
    const key = apiKey.trim();
    if (!key) return;

    // Save API key
    await settingsStore.saveApiKey(selectedCloudProvider, key);
    apiKey = '';

    // Auto-select provider if none is active
    if (!activeProviderId || CLOUD_PROVIDERS.includes(activeProviderId)) {
      await settingsStore.setActiveProvider(selectedCloudProvider);
    }
  }

  async function handleDeleteCloudKey() {
    await settingsStore.deleteApiKey(selectedCloudProvider);

    // Clear active provider if deleted
    if (activeProviderId === selectedCloudProvider) {
      const configured = providers.filter(p => p.configured && p.id !== selectedCloudProvider);
      if (configured.length > 0) {
        await settingsStore.setActiveProvider(configured[0].id);
      }
    }
  }

  async function handleSelectLocalProvider(providerId: string) {
    await settingsStore.setActiveProvider(providerId);
  }

  async function handleGpuTypeChange(type: string) {
    try {
      await invoke('set_gpu_type', { gpuType: type });
      gpuType = type;
    } catch (e) {
      console.error('Failed to set GPU type:', e);
    }
  }

  function toggleShowApiKey() {
    showApiKey = !showApiKey;
  }

  async function handleModelChange(model: string) {
    const isCustom = model === 'custom';
    if (isCustom) {
      isCustomModel = true;
      selectedModel = 'custom';
    } else {
      isCustomModel = false;
      selectedModel = model;
      await invoke('set_provider_model', {
        provider: selectedCloudProvider,
        model,
        is_custom: false
      });
    }
  }

  async function handleCustomModelSave() {
    const customModel = customModelInput.trim();
    if (!customModel) return;

    await invoke('set_provider_model', {
      provider: selectedCloudProvider,
      model: customModel,
      is_custom: true
    });
    selectedModel = customModel;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div
  class="settings-backdrop"
  on:click={handleBackdropClick}
  on:keydown={(e) => e.key === 'Escape' && handleClose()}
  role="dialog"
  aria-modal="true"
  aria-labelledby="settings-title"
  tabindex="-1"
>
  <div class="settings-modal">
    <header class="settings-header">
      <h2 id="settings-title">Settings</h2>
      <button class="close-button" on:click={handleClose} title="Close">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6 6 18"/>
          <path d="m6 6 12 12"/>
        </svg>
      </button>
    </header>

    <div class="settings-content">
      <!-- Cloud Providers Section -->
      <section class="settings-section">
        <h3>Cloud AI Provider</h3>
        <p class="section-description">
          Configure your cloud AI provider. API keys are stored securely in Windows Credential Locker.
        </p>

        <div class="cloud-provider-config">
          <div class="form-group">
            <label for="provider-select" class="input-label">Provider</label>
            <select
              id="provider-select"
              bind:value={selectedCloudProvider}
              on:change={() => handleCloudProviderChange(selectedCloudProvider)}
              class="styled-select"
            >
              {#each providers.filter(p => CLOUD_PROVIDERS.includes(p.id)) as provider}
                <option value={provider.id}>{provider.name}</option>
              {/each}
            </select>
          </div>

          <div class="form-group">
            <label for="model-select" class="input-label">Model</label>
            <select
              id="model-select"
              bind:value={selectedModel}
              on:change={() => handleModelChange(selectedModel)}
              class="styled-select"
            >
              {#if recommendedModels[selectedCloudProvider]}
                {#each recommendedModels[selectedCloudProvider] as modelOption}
                  <option value={modelOption.id}>{modelOption.name}</option>
                {/each}
              {/if}
              <option value="custom">Custom model...</option>
            </select>

            {#if isCustomModel}
              <div class="custom-model-input">
                <input
                  type="text"
                  bind:value={customModelInput}
                  placeholder="Enter custom model name (e.g., gpt-4-turbo-2024-04-09)"
                  class="styled-input"
                  on:keypress={(e) => e.key === 'Enter' && handleCustomModelSave()}
                />
                <button
                  class="save-key-button"
                  on:click={handleCustomModelSave}
                  disabled={!customModelInput.trim()}
                >
                  Set Model
                </button>
              </div>
            {/if}
          </div>

          <div class="form-group">
            <label for="api-key-input" class="input-label">API Key</label>
            <div class="key-input-wrapper">
              <input
                id="api-key-input"
                type={showApiKey ? 'text' : 'password'}
                bind:value={apiKey}
                placeholder={providers.find(p => p.id === selectedCloudProvider)?.configured
                  ? 'Enter new key to update'
                  : 'Enter API key'}
                class="styled-input"
                on:keypress={(e) => e.key === 'Enter' && handleSaveCloudProvider()}
              />
              <button
                class="toggle-visibility"
                on:click={toggleShowApiKey}
                title={showApiKey ? 'Hide' : 'Show'}
              >
                {#if showApiKey}
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/>
                    <line x1="1" y1="1" x2="23" y2="23"/>
                  </svg>
                {:else}
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                  </svg>
                {/if}
              </button>
            </div>
          </div>

          <div class="key-actions">
            <button
              class="save-key-button"
              on:click={handleSaveCloudProvider}
              disabled={!apiKey.trim() || isLoading}
            >
              {providers.find(p => p.id === selectedCloudProvider)?.configured ? 'Update' : 'Save'}
            </button>

            {#if providers.find(p => p.id === selectedCloudProvider)?.configured}
              <button
                class="delete-key-button"
                on:click={handleDeleteCloudKey}
                disabled={isLoading}
              >
                Remove Key
              </button>
            {/if}
          </div>

          {#if providers.find(p => p.id === selectedCloudProvider)?.configured}
            <div class="provider-status-indicator configured">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                <polyline points="22 4 12 14.01 9 11.01"/>
              </svg>
              <span>Configured and ready</span>
            </div>

            <button
              class="provider-select-button"
              class:active={activeProviderId === selectedCloudProvider}
              on:click={() => settingsStore.setActiveProvider(selectedCloudProvider)}
            >
              <span class="provider-radio">
                {#if activeProviderId === selectedCloudProvider}
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                    <circle cx="12" cy="12" r="8"/>
                  </svg>
                {/if}
              </span>
              <span>Use this provider</span>
            </button>
          {/if}
        </div>
      </section>

      <!-- Local Models Section -->
      <section class="settings-section">
        <h3>Local AI Models</h3>
        <p class="section-description">
          Run AI models locally on your device. No API key or internet connection required.
        </p>

        <div class="gpu-config">
          <div class="form-group">
            <label for="gpu-select" class="input-label">GPU Acceleration</label>
            <select
              id="gpu-select"
              bind:value={gpuType}
              on:change={() => handleGpuTypeChange(gpuType)}
              class="styled-select"
            >
              <option value="cpu">None (CPU only)</option>
              <option value="vulkan">Enabled (GPU Acceleration)</option>
            </select>
            <p class="config-hint">Requires a compatible GPU and drivers. Uses Vulkan for maximum compatibility.</p>
          </div>
        </div>

        <div class="providers-list">
          {#each providers.filter(p => isLocalModel(p.id)) as provider (provider.id)}
            <div
              class="provider-item"
              class:active={activeProviderId === provider.id}
            >
              <div class="provider-header">
                <button
                  class="provider-select"
                  on:click={() => handleSelectLocalProvider(provider.id)}
                >
                  <span class="provider-radio">
                    {#if activeProviderId === provider.id}
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                        <circle cx="12" cy="12" r="8"/>
                      </svg>
                    {/if}
                  </span>
                  <span class="provider-name">{provider.name}</span>
                </button>
              </div>
              <LocalModelSettings {provider} />
            </div>
          {/each}
        </div>
      </section>

      <section class="settings-section">
        <h3>Claude Desktop</h3>
        <p class="section-description">
          Connect HexStickyNote to Claude Desktop so Claude can create and manage your sticky notes.
        </p>

        <div class="claude-integration">
          {#if !claudeInstalled}
            <div class="claude-status not-installed">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="15" y1="9" x2="9" y2="15"/>
                <line x1="9" y1="9" x2="15" y2="15"/>
              </svg>
              <div>
                <p><strong>Claude Desktop not found</strong></p>
                <p class="claude-detail">Install Claude Desktop to enable this integration.</p>
              </div>
            </div>
          {:else if mcpConfigured}
            <div class="claude-status configured">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                <polyline points="22 4 12 14.01 9 11.01"/>
              </svg>
              <div>
                <p><strong>Connected to Claude Desktop</strong></p>
                <p class="claude-detail">Claude can create, read, update, and delete your sticky notes.</p>
              </div>
            </div>
            <button
              class="claude-remove-button"
              on:click={handleRemoveMcp}
              disabled={mcpLoading}
            >
              {mcpLoading ? 'Removing...' : 'Remove from Claude Desktop'}
            </button>
          {:else}
            <div class="claude-status available">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="16"/>
                <line x1="8" y1="12" x2="16" y2="12"/>
              </svg>
              <div>
                <p><strong>Claude Desktop detected</strong></p>
                <p class="claude-detail">Add HexStickyNote tools to Claude Desktop's configuration.</p>
              </div>
            </div>
            <button
              class="claude-setup-button"
              on:click={handleSetupMcp}
              disabled={mcpLoading}
            >
              {mcpLoading ? 'Setting up...' : 'Add to Claude Desktop'}
            </button>
          {/if}
        </div>
      </section>

      <section class="settings-section">
        <h3>Security & Privacy</h3>
        <div class="security-info">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <div>
            <p><strong>Your data is secure</strong></p>
            <p class="security-detail">
              API keys are encrypted in Windows Credential Locker.
              Local models run completely offline on your device.
              Your data never leaves your computer.
            </p>
          </div>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .settings-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    backdrop-filter: blur(4px);
    pointer-events: auto;
  }

  .settings-modal {
    background: var(--bg-secondary);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    width: 100%;
    max-width: 560px;
    max-height: 80vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-lg);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  .settings-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0;
  }

  .close-button {
    background: transparent;
    color: var(--text-secondary);
    padding: 0.375rem;
    border-radius: 6px;
    display: flex;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .close-button:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }

  .settings-section {
    margin-bottom: 2rem;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .settings-section h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 0.5rem;
  }

  .section-description {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin: 0 0 1rem;
    line-height: 1.5;
  }

  .providers-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .provider-item {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1rem;
    transition: border-color var(--transition-fast);
  }

  .provider-item.active {
    border-color: var(--accent-primary);
  }

  .provider-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .provider-select {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: transparent;
    color: var(--text-primary);
    padding: 0;
  }

  .provider-select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .provider-radio {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-primary);
  }

  .provider-item.active .provider-radio {
    border-color: var(--accent-primary);
  }

  .provider-name {
    font-weight: 500;
  }


  .key-input-wrapper {
    display: flex;
    gap: 0.5rem;
  }

  .key-input-wrapper .styled-input {
    flex: 1;
    font-family: 'Courier New', monospace;
  }

  .toggle-visibility {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary);
    padding: 0 0.875rem;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    display: flex;
    align-items: center;
    transition: all 0.2s ease;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .toggle-visibility:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.2);
    color: var(--text-primary);
  }

  .key-actions {
    display: flex;
    gap: 0.5rem;
  }

  .save-key-button,
  .delete-key-button {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    border-radius: 6px;
    font-weight: 500;
    transition: background var(--transition-fast);
  }

  .save-key-button {
    background: var(--accent-primary);
    color: white;
  }

  .save-key-button:hover:not(:disabled) {
    background: var(--accent-secondary);
  }

  .save-key-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .delete-key-button {
    background: transparent;
    color: #ef4444;
    border: 1px solid #ef4444;
  }

  .delete-key-button:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.1);
  }

  .claude-integration {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .claude-status {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: 8px;
    align-items: flex-start;
  }

  .claude-status p {
    margin: 0;
  }

  .claude-status.not-installed {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .claude-status.not-installed p:first-child {
    color: var(--text-primary);
  }

  .claude-status.configured {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .claude-status.configured p:first-child {
    color: var(--text-primary);
  }

  .claude-status.available {
    background: rgba(99, 102, 241, 0.1);
    border: 1px solid rgba(99, 102, 241, 0.2);
    color: var(--accent-primary);
  }

  .claude-status.available p:first-child {
    color: var(--text-primary);
  }

  .claude-detail {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  .claude-setup-button {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    border-radius: 6px;
    font-weight: 500;
    background: var(--accent-primary);
    color: white;
    transition: background var(--transition-fast);
    align-self: flex-start;
  }

  .claude-setup-button:hover:not(:disabled) {
    background: var(--accent-secondary);
  }

  .claude-setup-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .claude-remove-button {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    border-radius: 6px;
    font-weight: 500;
    background: transparent;
    color: #ef4444;
    border: 1px solid #ef4444;
    transition: background var(--transition-fast);
    align-self: flex-start;
  }

  .claude-remove-button:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.1);
  }

  .claude-remove-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .security-info {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: 8px;
    color: #22c55e;
  }

  .security-info p {
    margin: 0;
  }

  .security-info p:first-child {
    color: var(--text-primary);
  }

  .security-detail {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }

  /* GPU Configuration */
  .gpu-config {
    margin-bottom: 1.5rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.25rem;
  }

  /* Cloud Provider Configuration */
  .cloud-provider-config {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.25rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  /* Styled Select - Glass Morphism */
  .styled-select {
    width: 100%;
    padding: 0.625rem 0.875rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .styled-select:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .styled-select:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    background: rgba(255, 255, 255, 0.08);
  }

  .styled-select option {
    background-color: #1a1a24;
    color: white;
  }

  /* Styled Input - Glass Morphism */
  .styled-input {
    width: 100%;
    padding: 0.625rem 0.875rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.875rem;
    transition: all 0.2s ease;
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .styled-input:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .styled-input:focus {
    outline: none;
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    background: rgba(255, 255, 255, 0.08);
  }

  .styled-input::placeholder {
    color: var(--text-muted);
  }

  .custom-model-input {
    margin-top: 0.5rem;
    display: flex;
    gap: 0.5rem;
  }

  .custom-model-input .styled-input {
    flex: 1;
    font-family: 'Courier New', monospace;
  }

  .custom-model-input .save-key-button {
    flex-shrink: 0;
    padding: 0.625rem 1rem;
  }

  .provider-status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .provider-status-indicator.configured {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .provider-select-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 0.875rem;
    font-weight: 500;
    transition: all var(--transition-fast);
    cursor: pointer;
    width: 100%;
    justify-content: center;
  }

  .provider-select-button:hover {
    background: var(--bg-hover);
    border-color: rgba(99, 102, 241, 0.3);
    color: var(--text-primary);
  }

  .provider-select-button.active {
    background: rgba(99, 102, 241, 0.1);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }

  .provider-select-button .provider-radio {
    width: 18px;
    height: 18px;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-primary);
    transition: border-color var(--transition-fast);
  }

  .provider-select-button.active .provider-radio {
    border-color: var(--accent-primary);
  }
</style>
