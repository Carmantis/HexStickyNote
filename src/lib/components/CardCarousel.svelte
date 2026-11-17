<script lang="ts">
  /**
   * Card Carousel - 3D rotating carousel display
   *
   * The cards stand on a ring seen slightly from above. Every card faces the
   * viewer; depth comes from perspective, so the cards behind the front one
   * show smaller, blurred and see-through. Only the front card is sharp and
   * can be opened; clicking another card turns the ring to it.
   */

  import { cardStore, editingCard, visibleCards, type Card } from "$lib/stores/cardStore";
  import NoteCard from "./Card.svelte";
  import NoteGrid from "./NoteGrid.svelte";
  import { onMount, tick } from "svelte";
  import { tweened } from "svelte/motion";
  import { cubicIn, cubicOut } from "svelte/easing";

  $: cards = $visibleCards;
  $: isLoading = $cardStore.isLoading;
  $: editing = $editingCard;

  let currentIndex = 0;
  let rotation = 0;
  let previousEditing: Card | null = null;
  let oldCardLength = 0;

  /** Duration of the carousel folding in / tiles flying back (ms) */
  const FOLD_MS = 380;
  const GRID_LEAVE_MS = 320;

  /** What is on screen; follows the store's layout through the animations */
  let shown: "carousel" | "grid" = $cardStore.layout;
  /** Grid tiles flying back into the middle */
  let gridLeaving = false;
  let switching = false;

  /** Ring angle shown on screen, easing towards `rotation` (degrees) */
  const spin = tweened(0, { duration: 600, easing: cubicOut });
  /** 0 = ring spread out, 1 = cards pulled into the middle (grid transition) */
  const fold = tweened(0, { duration: FOLD_MS, easing: cubicIn });

  /** Size of the area the ring is drawn in */
  let viewWidth = 900;
  let viewHeight = 700;

  $: wanted = $cardStore.layout;
  $: if (wanted !== shown && !switching) switchLayout(wanted);

  // Creating or opening a note while the grid is up goes straight to the editor
  $: if (editing && shown === "grid") {
    shown = "carousel";
    fold.set(0, { duration: 0 });
    gridLeaving = false;
    cardStore.setLayout("carousel");
  }

  const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  const nextFrame = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

  async function switchLayout(target: "carousel" | "grid") {
    switching = true;
    try {
      if (target === "grid") {
        await fold.set(1);
        // A note was opened for editing meanwhile: stay in the carousel
        if (editing || $cardStore.layout !== "grid") {
          fold.set(0, { duration: 0 });
          return;
        }
        shown = "grid";
      } else {
        gridLeaving = true;
        await wait(GRID_LEAVE_MS);
        gridLeaving = false;

        // Put the picked card in front without spinning there, then unfold
        await fold.set(1, { duration: 0 });
        const opening = focusPickedCard();
        shown = "carousel";
        await tick();
        await nextFrame();
        await fold.set(0, { duration: 480, easing: cubicOut });
        // A tile that was clicked opens straight into the editor
        if (opening) cardStore.enterEditMode(opening);
      }
    } finally {
      // A layout toggled again during the animation is picked up by `$: if (wanted ...)`
      switching = false;
    }
  }

  /** Bring the card picked in the grid to the front; its id if it should open */
  function focusPickedCard(): string | null {
    const id = $cardStore.focusCardId;
    const opens = $cardStore.focusOpens;
    if (!id) return null;
    cardStore.clearFocus();
    const index = cards.findIndex((c) => c.id === id);
    if (index !== -1) {
      currentIndex = index;
      faceCard(index, true);
      return opens ? id : null;
    }
    return null;
  }

  /**
   * Bring card `index` to the front through the nearest turn of the ring.
   * The ring's angle keeps growing as it is turned (-450° is the same place
   * as -90°); setting the plain angle would spin it all the way back.
   */
  function faceCard(index: number, instant = false) {
    const target = -index * angleIncrement;
    const turns = Math.round((rotation - target) / 360);
    setRotation(target + turns * 360, instant);
  }

  function setRotation(value: number, instant = false) {
    rotation = value;
    spin.set(value, instant ? { duration: 0 } : undefined);
  }

  // Calculate rotation angle based on number of cards
  $: angleIncrement = cards.length > 0 ? 360 / cards.length : 120;

  // ---- Ring geometry (px) -------------------------------------------------
  const PERSPECTIVE = 1100;
  $: cardWidth = Math.round(Math.min(520, Math.max(220, viewWidth * 0.36)));
  $: cardHeight = Math.round(Math.min(500, Math.max(160, viewHeight - 170)));
  /** Half-width of the ring; the side cards stay inside the view */
  $: radiusX = Math.min(viewWidth * 0.33, cardWidth * 1.2);
  /** Depth of the ring: the back card is this far behind the front one, twice */
  $: radiusZ = radiusX * 1.35;
  /** How much higher the back of the ring is, as if seen from above */
  $: lift = cardHeight * 0.42;

  interface Placement {
    transform: string;
    opacity: number;
    filter: string;
    zIndex: number;
  }

  /** Where card `i` is on the ring when the ring is turned to `angle` */
  function place(i: number, angle: number, folded: number, active: boolean): Placement {
    const theta = ((i * angleIncrement + angle) * Math.PI) / 180;
    const depth = (1 - Math.cos(theta)) / 2; // 0 = front, 1 = back
    const spread = 1 - folded;

    const x = radiusX * Math.sin(theta) * spread;
    const y = (-lift * depth + lift * 0.3) * spread;
    const z = -2 * radiusZ * depth * spread;
    const scale = 1 - 0.75 * folded;

    const front = active && depth < 0.02;
    const opacity = (front ? 1 : Math.max(0.15, 0.5 - 0.35 * depth)) * spread;
    const blur = front ? 0 : 3.5 + 4 * depth;

    return {
      transform: `translate3d(${x.toFixed(1)}px, ${y.toFixed(1)}px, ${z.toFixed(1)}px) scale(${scale.toFixed(3)})`,
      opacity,
      filter: front ? "none" : `blur(${blur.toFixed(1)}px) grayscale(${Math.round(35 * depth + 15)}%)`,
      zIndex: Math.round((1 - depth) * 1000),
    };
  }

  // Watch for edit mode changes and card count changes to maintain alignment
  $: {
    if (editing) {
      previousEditing = editing;
    } else {
      // Logic runs when not editing (or just exited), or when cards change
      let targetIndex = currentIndex;
      let shouldRecalculate = false;

      // Case 1: Just exited edit mode
      if (previousEditing) {
        const index = cards.findIndex((c) => c.id === previousEditing?.id);
        if (index !== -1) {
          targetIndex = index;
        } else if (cards.length > 0) {
          // Card was deleted, clamp index
          targetIndex = Math.min(currentIndex, cards.length - 1);
        }
        previousEditing = null;
        shouldRecalculate = true;
      }

      // Case 2: Card count changed (e.g. added new card)
      if (cards.length !== oldCardLength) {
        if (cards.length > oldCardLength) {
          // If added, usually we want to show the new card (which is at the end)
          // But if we just exited edit mode (handled above), targetIndex is already set.
          // If added externally, maybe we stay put?
          // For now, let's respect targetIndex unless invalid.
        }
        targetIndex = Math.min(targetIndex, Math.max(0, cards.length - 1));
        oldCardLength = cards.length;
        shouldRecalculate = true;
      }

      if (shouldRecalculate) {
        currentIndex = targetIndex;
        // Snap rotation to align the current card perfectly
        faceCard(currentIndex);
      }
    }
  }

  function rotateCarousel(direction: "next" | "prev") {
    turnTo(direction === "next" ? currentIndex + 1 : currentIndex - 1);
  }

  /** Turn the ring the short way round so that card `index` is in front */
  function turnTo(index: number) {
    const n = cards.length;
    if (n === 0) return;
    let steps = (((index - currentIndex) % n) + n) % n;
    if (steps > n / 2) steps -= n;
    currentIndex = (((index % n) + n) % n);
    setRotation(rotation - steps * angleIncrement);
  }

  /** A card behind the front one is not opened by a click; the ring turns to it */
  function handleItemClick(event: MouseEvent, index: number) {
    if (index === currentIndex) return;
    event.stopPropagation();
    event.preventDefault();
    turnTo(index);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (editing || shown !== "carousel") return; // Don't navigate while editing

    // Arrow keys in a text field (e.g. the assistant chat) move the cursor
    const target = event.target as HTMLElement | null;
    if (target?.closest('input, textarea, [contenteditable="true"]')) return;

    if (event.key === "ArrowLeft") {
      rotateCarousel("prev");
    } else if (event.key === "ArrowRight") {
      rotateCarousel("next");
    }
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeydown);
    return () => document.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="carousel">
  {#if isLoading}
    <div class="carousel-loading">
      <div class="spinner"></div>
      <p>Loading cards...</p>
    </div>
  {:else if cards.length === 0}
    <div class="carousel-empty">
      <div class="empty-icon">
        <svg
          width="48"
          height="48"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <path d="M12 8v8" />
          <path d="M8 12h8" />
        </svg>
      </div>
      {#if $cardStore.cards.length > 0}
        <h3>No notes in this folder</h3>
        <p>Create a note here, or pick another folder from the top bar</p>
      {:else}
        <h3>No notes yet</h3>
        <p>Create your first note to get started</p>
      {/if}
    </div>
  {:else if shown === "grid"}
    <NoteGrid {cards} leaving={gridLeaving} />
  {:else}
    <div class="carousel-3d" class:folded={$fold > 0.5}>
      <!-- Navigation Arrows -->
      <button
        class="nav-arrow nav-arrow-left"
        on:click={() => rotateCarousel("prev")}
        disabled={!!editing}
        title="Previous"
      >
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>

      <!-- 3D Carousel Container -->
      {#if !editing}
        <div
          class="carousel-viewport"
          bind:clientWidth={viewWidth}
          bind:clientHeight={viewHeight}
          style="perspective: {PERSPECTIVE}px"
        >
          <!-- Cards are only moved, never turned: WebKitGTK mirrors or culls cards
               rotated away from the viewer, so every card faces the front. -->
          <div class="carousel-stage">
            {#each cards as card, i (card.id)}
              {@const at = place(i, $spin, $fold, i === currentIndex)}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
              <div
                class="carousel-item"
                class:active={i === currentIndex}
                on:click|capture={(e) => handleItemClick(e, i)}
                style="width: {cardWidth}px; height: {cardHeight}px; transform: {at.transform}; opacity: {at.opacity}; filter: {at.filter}; z-index: {at.zIndex};"
              >
                <NoteCard {card} />
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <!-- Editing mode - show card in center -->
        <div class="carousel-viewport">
          <div class="edit-container">
            <NoteCard card={editing} />
          </div>
        </div>
      {/if}

      <button
        class="nav-arrow nav-arrow-right"
        on:click={() => rotateCarousel("next")}
        disabled={!!editing}
        title="Next"
      >
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
      </button>

      <!-- Indicator dots -->
      {#if !editing}
        <div class="carousel-indicators">
          {#each cards as card, i}
            <button
              class="indicator"
              class:active={i === currentIndex}
              on:click={() => turnTo(i)}
              title={`Card ${i + 1}`}
            />
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .carousel {
    height: 100%;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .carousel-loading,
  .carousel-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 1rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-icon {
    color: var(--text-muted);
    opacity: 0.5;
  }

  .carousel-empty h3 {
    font-size: 1.25rem;
    color: var(--text-primary);
    margin: 0;
  }

  .carousel-empty p {
    margin: 0;
    color: var(--text-muted);
  }

  /* 3D Carousel */
  .carousel-3d {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .carousel-viewport {
    width: 100%;
    height: 100%;
    perspective: 1200px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .carousel-stage {
    width: 100%;
    height: 100%;
    position: relative;
    transform-style: preserve-3d;
    transition: transform 0.6s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .carousel-item {
    position: absolute;
    left: 50%;
    top: 50%;
    /* Size, place, opacity and blur are set per card from the ring geometry */
    translate: -50% -50%;
    transform-style: flat;
    will-change: transform, opacity, filter;
    pointer-events: auto;
    cursor: pointer;
  }

  /* Hide the grid transition's half-way state from the arrows and dots */
  .folded .nav-arrow,
  .folded .carousel-indicators {
    opacity: 0 !important;
    pointer-events: none;
  }

  /* The front card is solid and readable */
  .carousel-item.active :global(.card) {
    background: #12121a !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
    border-color: var(--accent-primary);
    box-shadow: 0 0 40px rgba(0, 0, 0, 0.6);
    transform: translateZ(0);
  }

  /* Cards behind it must not react like the front one */
  .carousel-item:not(.active) :global(.card:hover) {
    transform: translateZ(0);
    border-color: rgba(255, 255, 255, 0.1);
  }

  .carousel-item :global(.card) {
    height: 100%;
    width: 100%;
  }

  /* Edit container - matches the visual size of the magnified carousel card */
  .edit-container {
    width: clamp(300px, 55vw, 800px);
    height: clamp(200px, min(calc(80vh - 6rem), calc(100% - 2rem)), 750px);
    display: flex;
    align-items: stretch;
    justify-content: center;
  }

  .edit-container :global(.card) {
    width: 100%;
    height: 100%;
    background: #12121a !important;
    backdrop-filter: none !important;
    -webkit-backdrop-filter: none !important;
  }

  /* Navigation Arrows */
  .nav-arrow {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(26, 26, 36, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 50%;
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    z-index: 10;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    transition:
      background var(--transition-fast),
      color var(--transition-fast),
      border-color var(--transition-fast),
      transform var(--transition-fast),
      opacity var(--transition-fast);
  }

  .nav-arrow:hover:not(:disabled) {
    background: rgba(99, 102, 241, 0.2);
    border-color: var(--accent-primary);
    color: var(--accent-primary);
    transform: translateY(-50%) scale(1.1);
  }

  .nav-arrow:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .nav-arrow-left {
    left: 15vw;
  }

  .nav-arrow-right {
    right: 15vw;
  }

  /* Indicators */
  .carousel-indicators {
    transition: opacity 0.3s ease;
    position: absolute;
    top: 1rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 0.75rem;
    z-index: 10;
  }

  .indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.2);
    border: none;
    cursor: pointer;
    transition:
      background var(--transition-fast),
      transform var(--transition-fast);
    padding: 0;
  }

  .indicator:hover {
    background: rgba(255, 255, 255, 0.4);
    transform: scale(1.2);
  }

  .indicator.active {
    background: var(--accent-primary);
    transform: scale(1.3);
  }
</style>
