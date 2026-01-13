<script lang="ts">
  /**
   * Card Component - Dual-mode card with view/edit states
   *
   * View Mode: Renders markdown as sanitized HTML
   * Edit Mode: Shows CodeMirror editor with raw markdown
   */

  import { onMount, onDestroy } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { cardStore, getCardMode, type Card } from '$lib/stores/cardStore';
  import Editor from './Editor.svelte';
  import AiPromptBar from './AiPromptBar.svelte';

  // Props
  export let card: Card;

  const DEFAULT_TEXT = '# New Note\n\nStart typing...';

  // Local state
  let editorContent = card.content;
  let cardElement: HTMLElement;
  let isAiStreaming = false;

  // Sync editor content if card prop updates from outside (e.g. backend tool refresh)
  $: if (card.content !== editorContent) {
    editorContent = card.content;
  }

  // Reactive: get card mode from store
  $: mode = getCardMode(card.id);

  // Parse and sanitize markdown for view mode
  $: renderedHtml = DOMPurify.sanitize(
    marked.parse(card.content, { async: false }) as string,
    { USE_PROFILES: { html: true } }
  );

  // Handle card click -> enter edit mode
  function handleCardClick(event: MouseEvent) {
    // Stop bubbling to prevent immediate close via document listener
    event.stopPropagation();

    // Don't enter edit mode if clicking on a link
    const target = event.target as HTMLElement;
    if (target && target.tagName === 'A') {
      return;
    }

    cardStore.enterEditMode(card.id);
    editorContent = card.content;
  }

  // Handle enter key on card -> enter edit mode
  function handleCardEnter(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.stopPropagation();
      cardStore.enterEditMode(card.id);
      editorContent = card.content;
    }
  }

  // Handle click outside -> exit edit mode
  function handleClickOutside(event: MouseEvent) {
    if ($mode === 'edit' && cardElement && !cardElement.contains(event.target as Node)) {
      cardStore.exitEditMode(card.id);
    }
  }

  // Handle editor content changes
  function handleEditorChange(content: string) {
    editorContent = content;
    cardStore.updateCardContent(card.id, content);
  }

  // Handle AI response chunks
  function handleAiChunk(chunk: string) {
    // If starting a new stream, REPLACE content instead of appending
    if (!isAiStreaming) {
      isAiStreaming = true;
      editorContent = chunk;
      cardStore.updateCardContent(card.id, chunk);
    } else {
      editorContent += chunk;
      cardStore.appendToCard(card.id, chunk);
    }
  }

  function handleAiDone() {
    isAiStreaming = false;
  }

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    // Escape to exit edit mode
    if (event.key === 'Escape' && $mode === 'edit') {
      cardStore.exitEditMode(card.id);
    }

    // Ctrl+S to save
    if (event.key === 's' && (event.ctrlKey || event.metaKey) && $mode === 'edit') {
      event.preventDefault();
      cardStore.exitEditMode(card.id);
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
    document.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
    document.removeEventListener('keydown', handleKeydown);
  });
</script>

<article
  class="card"
  class:editing={$mode === 'edit'}
  bind:this={cardElement}
  role="button"
  tabindex="0"
  on:click={handleCardClick}
  on:keydown={handleCardEnter}
>
  {#if $mode === 'view'}
    <!-- View Mode: Rendered HTML -->
    <div class="card-content markdown-content">
      {@html renderedHtml}
    </div>
  {:else}
    <!-- Edit Mode: CodeMirror Editor -->
    <div class="card-editor" on:click|stopPropagation>
      <Editor
        content={editorContent}
        on:change={(e) => handleEditorChange(e.detail)}
      />
      <AiPromptBar
        context={editorContent}
        on:chunk={(e) => handleAiChunk(e.detail)}
        on:done={handleAiDone}
        on:error={handleAiDone}
      />
    </div>
  {/if}

  <footer class="card-footer">
    <time class="card-time">
      {new Date(card.updated_at * 1000).toLocaleDateString()}
    </time>
  </footer>
</article>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--border-radius);
    padding: 1rem;
    cursor: pointer;
    transition:
      transform var(--transition-fast),
      box-shadow var(--transition-fast),
      border-color var(--transition-fast);
    min-height: 120px;
    display: flex;
    flex-direction: column;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .card:hover:not(.editing) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
    border-color: var(--accent-primary);
  }

  .card.editing {
    cursor: default;
    box-shadow: var(--shadow-lg), 0 0 20px var(--accent-glow);
    border-color: var(--accent-primary);
    min-height: 250px;
    max-height: 70vh;
    overflow: hidden;
  }

  .card-content {
    flex: 1;
    overflow-y: auto;
    user-select: text;
    padding-right: 0.5rem; /* Space for scrollbar */
  }

  .card-editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow: hidden;
    height: 100%;
    min-height: 0; /* Important for flex child scrolling */
  }

  .card-footer {
    margin-top: auto;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
  }

  .card-time {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
