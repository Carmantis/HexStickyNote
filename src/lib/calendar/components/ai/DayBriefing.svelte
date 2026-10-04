<svelte:options runes={true} />

<script lang="ts">
  interface Props {
    content: string | null;
    loading: boolean;
    onRefresh?: () => void;
  }

  let { content, loading, onRefresh }: Props = $props();
</script>

{#if loading}
  <div class="briefing briefing--loading">
    <div class="briefing-skeleton"></div>
    <div class="briefing-skeleton short"></div>
  </div>
{:else if content}
  <div class="briefing">
    <p class="briefing-text">{content}</p>
    {#if onRefresh}
      <button class="refresh-btn" onclick={onRefresh} title="Päivitä yhteenveto" aria-label="Päivitä yhteenveto">↻</button>
    {/if}
  </div>
{/if}

<style>
  .briefing {
    max-width: 360px;
    padding: var(--space-2) var(--space-3);
    background-color: var(--color-surface-elevated);
    border-radius: var(--radius-md);
    border-left: 3px solid var(--color-accent);
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
  }

  .briefing-text {
    font-size: var(--text-sm);
    color: var(--color-text-muted);
    margin: 0;
    line-height: 1.6;
    flex: 1;
  }

  .refresh-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: var(--text-base);
    padding: 0 0 0 var(--space-2);
    opacity: 0.5;
    flex-shrink: 0;
    align-self: flex-start;
    line-height: 1;
  }

  .refresh-btn:hover {
    opacity: 1;
    color: var(--color-accent);
  }

  .briefing--loading {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .briefing-skeleton {
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

  .briefing-skeleton.short {
    width: 60%;
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
