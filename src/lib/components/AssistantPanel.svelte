<script lang="ts">
  /**
   * Assistant Panel - chat with the assistant that can use notes, the calendar
   * and time tracking. Changes it proposes are shown as cards to approve or
   * decline before they are made.
   */

  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { assistantStore, type AssistantContext } from '$lib/stores/assistantStore';
  import { activeModel } from '$lib/stores/settingsStore';
  import { cardStore, editingCard } from '$lib/stores/cardStore';

  export let view: string;

  const dispatch = createEventDispatcher<{ close: void }>();

  const SUGGESTIONS = [
    "What's on my calendar this week?",
    'How many hours did I track today?',
    'Write a note summarising this week’s events',
  ];

  let input = '';
  let messagesElement: HTMLElement;
  let inputElement: HTMLTextAreaElement;
  let savedEntries = new Set<number>();

  $: state = $assistantStore;
  $: isReady = $activeModel?.source === 'ollama';
  $: canSend = isReady && !state.isBusy && state.pending.length === 0;
  $: context = { view, open_note_id: $editingCard?.id ?? null } satisfies AssistantContext;

  // Keep the newest message in view
  $: if (messagesElement && (state.entries.length || state.pending.length || state.isBusy)) {
    tick().then(() => messagesElement?.scrollTo({ top: messagesElement.scrollHeight }));
  }

  // Ready to type as soon as the panel opens
  onMount(() => inputElement?.focus());

  function render(markdown: string): string {
    return DOMPurify.sanitize(marked.parse(markdown, { async: false }) as string, {
      USE_PROFILES: { html: true },
    });
  }

  function send(text = input) {
    if (!canSend || !text.trim()) return;
    input = '';
    assistantStore.send(text, context);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  }

  async function saveAsNote(index: number, text: string) {
    const card = await cardStore.createCard(text);
    if (card) {
      savedEntries = new Set(savedEntries).add(index);
    }
  }

  function newChat() {
    assistantStore.reset();
    savedEntries = new Set();
  }
</script>

<aside class="assistant-panel" aria-label="Assistant">
  <header class="panel-header">
    <div class="panel-title">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
      </svg>
      <span>Assistant</span>
      {#if $activeModel}
        <span class="model-name" title="Change the model in Settings">{$activeModel.name}</span>
      {/if}
    </div>
    <div class="panel-actions">
      <button class="icon-button" on:click={newChat} title="New chat" disabled={state.isBusy}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </button>
      <button class="icon-button" on:click={() => dispatch('close')} title="Close assistant">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>
  </header>

  <div class="messages" bind:this={messagesElement}>
    {#if !isReady}
      <p class="notice">
        The assistant needs an Ollama model that supports tools. Choose one under
        <strong>Settings → Local AI Models</strong>.
      </p>
    {:else if state.entries.length === 0}
      <div class="welcome">
        <p>Ask about your notes, calendar and tracked time, or ask for changes. You approve every change before it is made.</p>
        <div class="suggestions">
          {#each SUGGESTIONS as suggestion}
            <button class="suggestion" on:click={() => send(suggestion)}>{suggestion}</button>
          {/each}
        </div>
      </div>
    {/if}

    {#each state.entries as entry, i}
      {#if entry.kind === 'user'}
        <div class="bubble user">{entry.text}</div>
      {:else if entry.kind === 'assistant'}
        <div class="bubble assistant">
          <div class="markdown">{@html render(entry.text)}</div>
          <button
            class="save-note"
            on:click={() => saveAsNote(i, entry.text)}
            disabled={savedEntries.has(i)}
          >
            {savedEntries.has(i) ? 'Saved as note' : 'Save as note'}
          </button>
        </div>
      {:else if entry.kind === 'action'}
        <div class="action" class:failed={!entry.record.ok}>
          <span class="action-icon">{entry.record.ok ? '✓' : '✕'}</span>
          <span>
            {entry.record.summary}{#if entry.record.error}<span class="action-error"> – {entry.record.error}</span>{/if}
          </span>
        </div>
      {:else}
        <div class="bubble error">{entry.text}</div>
      {/if}
    {/each}

    {#each state.pending as action, i}
      <div class="confirm-card">
        <p class="confirm-label">Confirm change</p>
        <p class="confirm-summary">{action.summary}</p>
        {#if state.decisions[i] === null}
          <div class="confirm-buttons">
            <button class="approve" on:click={() => assistantStore.decide(i, true, context)} disabled={state.isBusy}>
              Approve
            </button>
            <button class="decline" on:click={() => assistantStore.decide(i, false, context)} disabled={state.isBusy}>
              Decline
            </button>
          </div>
        {:else}
          <p class="confirm-decided">{state.decisions[i] ? 'Approved' : 'Declined'}</p>
        {/if}
      </div>
    {/each}

    {#if state.isBusy}
      <div class="thinking">
        <span class="dot"></span><span class="dot"></span><span class="dot"></span>
      </div>
    {/if}
  </div>

  <footer class="composer">
    <textarea
      bind:this={inputElement}
      bind:value={input}
      on:keydown={handleKeydown}
      rows="2"
      placeholder={!isReady
        ? 'Select an Ollama model in Settings'
        : state.pending.length > 0
          ? 'Approve or decline the change first'
          : 'Ask the assistant…'}
      disabled={!isReady}
    ></textarea>
    <button class="send" on:click={() => send()} disabled={!canSend || !input.trim()} title="Send">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M22 2 11 13" />
        <path d="m22 2-7 20-4-9-9-4 20-7z" />
      </svg>
    </button>
  </footer>
</aside>

<style>
  .assistant-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: rgba(18, 18, 26, 0.96);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    pointer-events: auto;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .panel-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    font-weight: 600;
    color: var(--text-primary);
  }

  .panel-title svg {
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .model-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.75rem;
    font-weight: 400;
    color: var(--text-muted);
  }

  .panel-actions {
    display: flex;
    gap: 0.25rem;
  }

  .icon-button {
    display: flex;
    padding: 0.4rem;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 6px;
  }

  .icon-button:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem;
    user-select: text;
  }

  .notice,
  .welcome p {
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .suggestions {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-top: 0.75rem;
  }

  .suggestion {
    text-align: left;
    padding: 0.5rem 0.75rem;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.8125rem;
  }

  .suggestion:hover {
    border-color: var(--accent-primary);
  }

  .bubble {
    max-width: 92%;
    padding: 0.6rem 0.8rem;
    border-radius: 10px;
    font-size: 0.875rem;
    line-height: 1.5;
    word-break: break-word;
  }

  .bubble.user {
    align-self: flex-end;
    white-space: pre-wrap;
    background: var(--accent-primary);
    color: white;
  }

  .bubble.assistant {
    align-self: flex-start;
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
  }

  .bubble.error {
    align-self: stretch;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  .markdown :global(p) {
    margin: 0 0 0.5rem;
  }

  .markdown :global(p:last-child) {
    margin-bottom: 0;
  }

  .markdown :global(ul),
  .markdown :global(ol) {
    margin: 0.25rem 0 0.5rem 1.25rem;
  }

  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3) {
    font-size: 0.95rem;
    margin: 0.5rem 0 0.25rem;
  }

  .save-note {
    margin-top: 0.5rem;
    padding: 0;
    background: none;
    color: var(--accent-secondary);
    font-size: 0.75rem;
  }

  .save-note:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  .action {
    display: flex;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .action-icon {
    color: #22c55e;
  }

  .action.failed .action-icon,
  .action-error {
    color: #ef4444;
  }

  .confirm-card {
    padding: 0.75rem;
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid var(--accent-primary);
    border-radius: 10px;
  }

  .confirm-label {
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--accent-secondary);
  }

  .confirm-summary {
    margin: 0.25rem 0 0.6rem;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .confirm-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .confirm-buttons button {
    padding: 0.4rem 0.9rem;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
  }

  .approve {
    background: var(--accent-primary);
    color: white;
  }

  .decline {
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .confirm-buttons button:disabled {
    opacity: 0.5;
  }

  .confirm-decided {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .thinking {
    display: flex;
    gap: 0.3rem;
    padding: 0.5rem 0;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    animation: pulse 1.2s infinite ease-in-out;
  }

  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes pulse {
    0%, 80%, 100% { opacity: 0.3; }
    40% { opacity: 1; }
  }

  .composer {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem;
    border-top: 1px solid var(--border-color);
  }

  textarea {
    flex: 1;
    resize: none;
    padding: 0.5rem 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.875rem;
    user-select: text;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent-primary);
  }

  .send {
    align-self: flex-end;
    display: flex;
    padding: 0.55rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 8px;
  }

  .send:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
