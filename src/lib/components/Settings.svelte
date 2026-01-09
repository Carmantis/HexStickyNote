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
  import { settingsStore, activeProvider } from '$lib/stores/settingsStore';

  const dispatch = createEventDispatcher<{ close: void }>();

  let apiKeyInputs: Record<string, string> = {};
  let showApiKey: Record<string, boolean> = {};

  $: providers = $settingsStore.providers;
  $: activeProviderId = $settingsStore.activeProviderId;
  $: isLoading = $settingsStore.isLoading;

  onMount(() => {
    // Initialize empty API key inputs for each provider
    providers.forEach(p => {
      apiKeyInputs[p.id] = '';
      showApiKey[p.id] = false;
    });
  });

  function handleClose() {
    dispatch('close');
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  }

  async function handleSaveKey(providerId: string) {
    const key = apiKeyInputs[providerId];
    if (!key.trim()) return;

    await settingsStore.saveApiKey(providerId, key.trim());
    apiKeyInputs[providerId] = '';

    // Auto-select provider if none is active
    if (!activeProviderId) {
      await settingsStore.setActiveProvider(providerId);
    }
  }

  async function handleDeleteKey(providerId: string) {
    await settingsStore.deleteApiKey(providerId);

    // Clear active provider if deleted
    if (activeProviderId === providerId) {
      const configured = providers.filter(p => p.configured && p.id !== providerId);
      if (configured.length > 0) {
        await settingsStore.setActiveProvider(configured[0].id);
      }
    }
  }

  async function handleSelectProvider(providerId: string) {
    await settingsStore.setActiveProvider(providerId);
  }

  function toggleShowApiKey(providerId: string) {
    showApiKey[providerId] = !showApiKey[providerId];
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
      <section class="settings-section">
        <h3>AI Provider</h3>
        <p class="section-description">
          Select your preferred AI provider and enter your API key.
          Keys are stored securely in Windows Credential Locker.
        </p>

        <div class="providers-list">
          {#each providers as provider (provider.id)}
            <div
              class="provider-item"
              class:active={activeProviderId === provider.id}
              class:configured={provider.configured}
            >
              <div class="provider-header">
                <button
                  class="provider-select"
                  on:click={() => handleSelectProvider(provider.id)}
                  disabled={!provider.configured}
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

                <span class="provider-status" class:configured={provider.configured}>
                  {provider.configured ? 'Configured' : 'Not configured'}
                </span>
              </div>

              <div class="provider-key-form">
                <div class="key-input-wrapper">
                  <input
                    type={showApiKey[provider.id] ? 'text' : 'password'}
                    bind:value={apiKeyInputs[provider.id]}
                    placeholder={provider.configured ? 'Enter new key to update' : 'Enter API key'}
                    class="key-input"
                  />
                  <button
                    class="toggle-visibility"
                    on:click={() => toggleShowApiKey(provider.id)}
                    title={showApiKey[provider.id] ? 'Hide' : 'Show'}
                  >
                    {#if showApiKey[provider.id]}
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

                <div class="key-actions">
                  <button
                    class="save-key-button"
                    on:click={() => handleSaveKey(provider.id)}
                    disabled={!apiKeyInputs[provider.id]?.trim() || isLoading}
                  >
                    {provider.configured ? 'Update' : 'Save'}
                  </button>

                  {#if provider.configured}
                    <button
                      class="delete-key-button"
                      on:click={() => handleDeleteKey(provider.id)}
                      disabled={isLoading}
                    >
                      Remove
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      </section>

      <section class="settings-section">
        <h3>Security</h3>
        <div class="security-info">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <div>
            <p><strong>Your API keys are secure</strong></p>
            <p class="security-detail">
              Keys are encrypted and stored in Windows Credential Locker.
              They never leave your device and are never stored in plaintext files.
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

  .provider-status {
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.25rem 0.5rem;
    background: var(--bg-secondary);
    border-radius: 4px;
  }

  .provider-status.configured {
    color: #22c55e;
    background: rgba(34, 197, 94, 0.1);
  }

  .provider-key-form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .key-input-wrapper {
    display: flex;
    gap: 0.5rem;
  }

  .key-input {
    flex: 1;
    font-family: monospace;
    font-size: 0.875rem;
  }

  .toggle-visibility {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    padding: 0 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    display: flex;
    align-items: center;
  }

  .toggle-visibility:hover {
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
</style>
