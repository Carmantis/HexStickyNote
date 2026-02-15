<script lang="ts">
  /**
   * AI Prompt Bar - Input for AI commands
   *
   * Located at the bottom of the editor.
   * Sends prompts to the selected AI provider.
   */

  import { createEventDispatcher } from 'svelte';
  import { settingsStore, isAiReady, activeProvider } from '$lib/stores/settingsStore';

  // Props
  export let context: string = '';

  const dispatch = createEventDispatcher<{
    chunk: string;
    done: void;
    error: string;
  }>();

  let prompt = '';
  let inputElement: HTMLInputElement;

  $: isReady = $isAiReady;
  $: provider = $activeProvider;
  $: isStreaming = $settingsStore.isStreaming;

  async function handleSubmit() {
    if (!prompt.trim() || !isReady || isStreaming) return;

    const userPrompt = prompt;
    prompt = '';

    await settingsStore.invokeAiStream(
      userPrompt,
      context,
      (chunk) => dispatch('chunk', chunk),
      () => dispatch('done'),
      (error) => dispatch('error', error)
    );
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSubmit();
    }
  }
</script>

<div class="prompt-bar" class:disabled={!isReady}>
  <div class="prompt-input-wrapper">
    <span class="prompt-icon">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M12 1v6m0 6v10"/>
        <path d="m4.22 4.22 4.24 4.24m7.08 7.08 4.24 4.24"/>
        <path d="M1 12h6m6 0h10"/>
        <path d="m4.22 19.78 4.24-4.24m7.08-7.08 4.24-4.24"/>
      </svg>
    </span>

    <input
      type="text"
      bind:this={inputElement}
      bind:value={prompt}
      on:keydown={handleKeydown}
      placeholder={isReady
        ? `Ask ${provider?.name ?? 'AI'}...`
        : 'Configure AI provider in Settings'}
      disabled={!isReady || isStreaming}
      class="prompt-input"
    />

    <button
      class="prompt-submit"
      on:click={handleSubmit}
      disabled={!prompt.trim() || !isReady || isStreaming}
      title="Send prompt"
    >
      {#if isStreaming}
        <svg class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20"/>
        </svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 2 11 13"/>
          <path d="m22 2-7 20-4-9-9-4 20-7z"/>
        </svg>
      {/if}
    </button>
  </div>

  {#if isStreaming && $settingsStore.currentGpuInfo}
    <div class="gpu-indicator">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="2" y="2" width="20" height="20" rx="2" ry="2"/>
        <path d="M6 12h12M12 6v12"/>
      </svg>
      <span>Using {$settingsStore.currentGpuInfo}</span>
    </div>
  {/if}

  {#if $settingsStore.error}
    <p class="prompt-error">{$settingsStore.error}</p>
  {/if}
</div>

<style>
  .prompt-bar {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .prompt-bar.disabled {
    opacity: 0.6;
  }

  .prompt-input-wrapper {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 0.5rem 0.75rem;
    transition: border-color var(--transition-fast);
  }

  .prompt-input-wrapper:focus-within {
    border-color: var(--accent-primary);
  }

  .prompt-icon {
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .prompt-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 0.875rem;
    padding: 0;
  }

  .prompt-input::placeholder {
    color: var(--text-muted);
  }

  .prompt-input:disabled {
    cursor: not-allowed;
  }

  .prompt-submit {
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: 6px;
    padding: 0.375rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      background var(--transition-fast),
      opacity var(--transition-fast);
  }

  .prompt-submit:hover:not(:disabled) {
    background: var(--accent-secondary);
  }

  .prompt-submit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .prompt-error {
    font-size: 0.75rem;
    color: #ef4444;
    margin: 0;
  }

  .gpu-indicator {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.7rem;
    color: var(--accent-primary);
    padding: 0 0.25rem;
    opacity: 0.8;
  }
</style>
