<svelte:options runes={true} />

<script lang="ts">
  import { digest, loadWeekDigest } from '$lib/calendar/stores/ai.svelte';
  import { isoWeekKey, todayIso } from '$lib/calendar/utils/dateHelpers';

  interface Props {
    anchorDate: string;
    visible: boolean;
    onClose?: () => void;
  }

  let { anchorDate, visible, onClose }: Props = $props();

  const weekKey = $derived(isoWeekKey(new Date(anchorDate)));

  $effect(() => {
    if (visible) {
      loadWeekDigest(weekKey, anchorDate);
    }
  });
</script>

{#if visible}
  <aside class="ai-sidebar">
    <div class="sidebar-header">
      <h2 class="sidebar-title">Weekly Summary</h2>
      <span class="week-key">{weekKey}</span>
      <button class="close-btn" onclick={onClose} aria-label="Close sidebar">
        ✕
      </button>
    </div>

    <div class="sidebar-content">
      {#if digest.loading}
        <div class="loading-lines">
          {#each [80, 100, 60, 90, 70] as width}
            <div class="skeleton" style:width="{width}%"></div>
          {/each}
        </div>
      {:else if digest.week}
        <div class="digest-content">
          {@html renderMarkdown(digest.week.content)}
        </div>
        <p class="generated-at">
          Generated {new Date(digest.week.created_at).toLocaleString()}
          {#if digest.week.model}by {digest.week.model}{/if}
        </p>
      {:else}
        <p class="empty">No summary available yet. It will be generated automatically.</p>
      {/if}
    </div>
  </aside>
{/if}

<script context="module" lang="ts">
  // Minimal markdown-to-HTML for the digest (headings, bullets, bold).
  // Avoids adding a full markdown library dependency.
  function renderMarkdown(md: string): string {
    return md
      .replace(/^## (.+)$/gm, '<h3>$1</h3>')
      .replace(/^### (.+)$/gm, '<h4>$1</h4>')
      .replace(/^\* (.+)$/gm, '<li>$1</li>')
      .replace(/^- (.+)$/gm, '<li>$1</li>')
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      .replace(/(<li>.*<\/li>)/gs, '<ul>$1</ul>')
      .replace(/\n\n/g, '<br/><br/>');
  }
</script>

<style>
  .ai-sidebar {
    width: 320px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--color-border);
    background-color: var(--color-surface-elevated);
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .sidebar-title {
    font-size: var(--text-base);
    font-weight: 600;
    margin: 0;
    flex: 1;
  }

  .week-key {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--text-base);
    color: var(--color-text-muted);
    padding: var(--space-1);
    border-radius: var(--radius-sm);
  }

  .close-btn:hover {
    background-color: var(--color-surface-hover);
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
  }

  .loading-lines {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .skeleton {
    height: 12px;
    border-radius: 4px;
    background: linear-gradient(
      90deg,
      var(--color-border) 25%,
      var(--color-surface-hover) 50%,
      var(--color-border) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .digest-content {
    font-size: var(--text-sm);
    line-height: 1.7;
    color: var(--color-text);
  }

  .digest-content :global(h3) {
    font-size: var(--text-base);
    font-weight: 600;
    margin: var(--space-3) 0 var(--space-1);
    color: var(--color-accent);
  }

  .digest-content :global(h4) {
    font-size: var(--text-sm);
    font-weight: 600;
    margin: var(--space-2) 0 var(--space-1);
  }

  .digest-content :global(ul) {
    padding-left: var(--space-4);
    margin: var(--space-1) 0;
  }

  .digest-content :global(li) {
    margin-bottom: var(--space-1);
  }

  .generated-at {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    margin-top: var(--space-4);
    font-style: italic;
  }

  .empty {
    font-size: var(--text-sm);
    color: var(--color-text-muted);
    font-style: italic;
  }
</style>
