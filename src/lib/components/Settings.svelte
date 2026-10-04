<script lang="ts">
  /**
   * Settings Component - Local AI and integrations
   *
   * Allows users to:
   * - Select and download local AI models
   * - Configure GPU acceleration
   * - Connect Claude Desktop via MCP
   */

  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import LocalModelSettings from './LocalModelSettings.svelte';

  const dispatch = createEventDispatcher<{ close: void }>();

  // GPU type state
  let gpuType = 'cpu';

  // Claude Desktop MCP state
  let claudeInstalled = false;
  let mcpConfigured = false;
  let mcpLoading = false;

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
    try {
      const settings = await invoke<{ gpu_type: string }>('get_all_settings');
      gpuType = settings.gpu_type || 'cpu';
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

  async function handleGpuTypeChange(type: string) {
    try {
      await invoke('set_gpu_type', { gpuType: type });
      gpuType = type;
    } catch (e) {
      console.error('Failed to set GPU type:', e);
    }
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
      <!-- Local Models Section -->
      <section class="settings-section">
        <h3>Local AI Models</h3>
        <p class="section-description">
          Run AI models locally on your device. No account or internet connection required after download. To use Claude, connect it through Claude Desktop below.
        </p>

        <div class="config-box">
          <LocalModelSettings />
        </div>

        <div class="config-box">
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
            <p class="config-hint">Applies to downloaded models. Requires a compatible GPU and drivers; Ollama manages its own GPU use.</p>
          </div>
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
              Local models run completely offline on your device.
              Your notes are plain Markdown files that never leave your computer.
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
  .config-box {
    margin-bottom: 1.5rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.25rem;
  }

  .config-hint {
    margin: 0;
    font-size: 0.75rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  /* Form controls */
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
</style>
