<script lang="ts">
  /**
   * Note Grid - quick overview of many notes as small tiles
   *
   * The tiles fly out of the middle of the view when the grid opens and back
   * into it when it closes (`leaving`), so the carousel seems to fold into the
   * grid and back. Clicking a tile brings that note to the front of the carousel
   * and opens it for editing.
   */

  import { onMount } from "svelte";
  import { cardStore, type Card } from "$lib/stores/cardStore";

  export let cards: Card[];
  /** Play the closing animation */
  export let leaving = false;

  let query = "";
  let searchInput: HTMLInputElement;

  $: showFolder = $cardStore.folder === null;
  $: tiles = cards
    .map((card) => ({ card, ...summarize(card.content) }))
    .sort((a, b) => b.card.updated_at - a.card.updated_at);
  $: matches = filter(tiles, query);

  function summarize(content: string): { title: string; snippet: string } {
    const lines = content.split("\n").map((l) => l.trim());
    let titleIndex = lines.findIndex((l) => /^#+\s*\S/.test(l));
    if (titleIndex === -1) titleIndex = lines.findIndex((l) => l && !l.startsWith("---"));

    const title = titleIndex === -1 ? "Note" : lines[titleIndex].replace(/^#+\s*/, "");
    const snippet = lines
      .filter((_, i) => i !== titleIndex)
      .map((l) => l.replace(/^([-+*]|\d+\.)\s+(\[[ xX]\]\s*)?/, "")) // list markers
      .join(" ")
      .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1") // links and images: keep the text
      .replace(/[#>*_`~]/g, "")
      .replace(/\s+/g, " ")
      .trim()
      .slice(0, 160);
    return { title, snippet };
  }

  function filter<T extends { card: Card; title: string }>(items: T[], text: string): T[] {
    const words = text.toLowerCase().split(/\s+/).filter(Boolean);
    if (words.length === 0) return items;
    return items.filter(({ card, title }) => {
      const haystack = `${title}\n${card.content}\n${card.folder}`.toLowerCase();
      return words.every((w) => haystack.includes(w));
    });
  }

  function formatDate(seconds: number): string {
    return new Date(seconds * 1000).toLocaleDateString();
  }

  function open(card: Card) {
    cardStore.showInCarousel(card.id, true);
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && matches.length > 0) {
      open(matches[0].card);
    }
  }

  /**
   * Offsets a tile so that it starts (and ends, when leaving) in the middle of
   * the grid; the CSS animations move it between there and its own place.
   */
  function fromCenter(node: HTMLElement, index: number) {
    // Not `grid`: bind:this is set only after the tiles inside it are created
    const area = node.closest(".grid-scroll")?.getBoundingClientRect();
    if (!area) return;
    const tile = node.getBoundingClientRect();
    const dx = area.left + area.width / 2 - (tile.left + tile.width / 2);
    const dy = area.top + Math.min(area.height, 500) / 2 - (tile.top + tile.height / 2);
    node.style.setProperty("--dx", `${dx}px`);
    node.style.setProperty("--dy", `${dy}px`);
    // Ripple outwards, but keep long lists quick
    node.style.setProperty("--delay", `${Math.min(index, 16) * 18}ms`);
  }

  /**
   * Typing anywhere in the grid searches; Escape clears the search, then goes
   * back to the carousel.
   */
  function handleWindowKeydown(event: KeyboardEvent) {
    if (leaving || event.defaultPrevented) return;
    const target = event.target as HTMLElement | null;
    const inSearch = target === searchInput;
    if (!inSearch && target?.closest('input, textarea, select, [contenteditable="true"]')) return;

    if (event.key === "Escape") {
      // Handled here, so the page does not close the whole HUD
      event.preventDefault();
      if (query) query = "";
      else cardStore.setLayout("carousel");
      return;
    }
    if (!inSearch && event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      // Focusing during keydown lets the character land in the search box
      searchInput.focus({ preventScroll: true });
    }
  }

  onMount(() => {
    searchInput?.focus({ preventScroll: true });
    // Capture phase: runs before the page's Escape-closes-the-HUD handler
    window.addEventListener("keydown", handleWindowKeydown, true);
    return () => window.removeEventListener("keydown", handleWindowKeydown, true);
  });
</script>

<div class="note-grid" class:leaving>
  <div class="toolbar">
    <div class="search">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="7" />
        <path d="m20 20-3.5-3.5" />
      </svg>
      <input
        bind:this={searchInput}
        bind:value={query}
        on:keydown={handleSearchKeydown}
        placeholder="Search notes"
        aria-label="Search notes"
      />
      <span class="result-count">
        {matches.length === tiles.length ? `${tiles.length} notes` : `${matches.length} / ${tiles.length}`}
      </span>
    </div>
  </div>

  <div class="grid-scroll">
    {#if matches.length === 0}
      <p class="no-results">No notes match “{query}”</p>
    {:else}
      <div class="grid">
        {#each matches as { card, title, snippet }, i (card.id)}
          <button class="tile" use:fromCenter={i} on:click={() => open(card)} title={title}>
            <span class="tile-title">{title}</span>
            {#if snippet}
              <span class="tile-snippet">{snippet}</span>
            {/if}
            <span class="tile-footer">
              <time>{formatDate(card.updated_at)}</time>
              {#if showFolder && card.folder}
                <span class="tile-folder">{card.folder}</span>
              {/if}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .note-grid {
    width: 100%;
    max-width: calc(800px + 3rem);
    height: 100%;
    margin: 0 auto;
    padding: 1rem 1.5rem 1.5rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    animation: fade-in 0.3s ease both;
  }

  .note-grid.leaving .toolbar {
    animation: fade-out 0.2s ease both;
  }

  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.75rem;
    background: rgba(18, 18, 26, 0.85);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    color: var(--text-muted);
  }

  .search:focus-within {
    border-color: var(--accent-primary);
  }

  .search input {
    flex: 1;
    padding: 0.6rem 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .result-count {
    flex-shrink: 0;
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .grid-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0.25rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
  }

  @media (max-width: 900px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 620px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  .tile {
    box-sizing: border-box;
    height: 132px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.75rem 0.85rem;
    background: #12121a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    animation: gather-in 0.45s cubic-bezier(0.2, 0.8, 0.2, 1) var(--delay, 0ms) both;
    transition:
      border-color var(--transition-fast),
      box-shadow var(--transition-fast),
      translate var(--transition-fast);
  }

  .tile:hover,
  .tile:focus-visible {
    border-color: var(--accent-primary);
    box-shadow: 0 0 18px var(--accent-glow);
    translate: 0 -2px;
    outline: none;
  }

  .note-grid.leaving .tile {
    animation: gather-out 0.3s cubic-bezier(0.4, 0, 1, 1) both;
    pointer-events: none;
  }

  .tile-title {
    font-weight: 600;
    font-size: 0.9rem;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }

  .tile-snippet {
    flex-shrink: 0;
    font-size: 0.78rem;
    line-height: 1.4;
    /* WebKitGTK does not clamp inside a flex column; cap the height as well */
    max-height: calc(1.4em * 3);
    color: var(--text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }

  .tile-footer {
    margin-top: auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .tile-folder {
    padding: 0.1rem 0.4rem;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--accent-glow);
    color: var(--accent-secondary);
    border-radius: 4px;
  }

  .no-results {
    margin-top: 3rem;
    text-align: center;
    color: var(--text-muted);
  }

  @keyframes gather-in {
    from {
      opacity: 0;
      transform: translate(var(--dx, 0), var(--dy, 0)) scale(0.3);
    }
  }

  @keyframes gather-out {
    to {
      opacity: 0;
      transform: translate(var(--dx, 0), var(--dy, 0)) scale(0.3);
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @keyframes fade-out {
    to {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile,
    .note-grid.leaving .tile {
      animation-duration: 1ms;
    }
  }
</style>
