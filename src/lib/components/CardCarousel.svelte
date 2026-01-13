<script lang="ts">
  /**
   * Card Carousel - 3D rotating carousel display
   *
   * Shows cards in a 3D carousel with navigation arrows.
   * Center card is highlighted and larger.
   */

  import { cardStore, editingCard, type Card } from "$lib/stores/cardStore";
  import NoteCard from "./Card.svelte";
  import { onMount } from "svelte";

  $: cards = $cardStore.cards;
  $: isLoading = $cardStore.isLoading;
  $: editing = $editingCard;

  let currentIndex = 0;
  let rotation = 0;
  let previousEditing: Card | null = null;
  let oldCardLength = 0;

  // Calculate rotation angle based on number of cards
  $: angleIncrement = cards.length > 0 ? 360 / cards.length : 120;

  // Dynamic radius: Expands as more cards are added to prevent overlap
  // Base radius 450px, adds spacing per card
  $: radius = Math.max(450, cards.length * 40);

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
        rotation = -currentIndex * angleIncrement;
      }
    }
  }

  function rotateCarousel(direction: "next" | "prev") {
    if (direction === "next") {
      currentIndex = (currentIndex + 1) % cards.length;
      rotation -= angleIncrement;
    } else {
      currentIndex = currentIndex === 0 ? cards.length - 1 : currentIndex - 1;
      rotation += angleIncrement;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (editing) return; // Don't navigate while editing

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
      <h3>No notes yet</h3>
      <p>Create your first note to get started</p>
    </div>
  {:else}
    <div class="carousel-3d">
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
        <div class="carousel-viewport">
          <div class="carousel-stage" style="transform: rotateY({rotation}deg)">
            {#each cards as card, i (card.id)}
              <div
                class="carousel-item"
                class:active={i === currentIndex}
                style="transform: rotateY({i *
                  angleIncrement}deg) translateZ({radius}px) {i === currentIndex
                  ? 'scale(1.05)'
                  : ''}"
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
              on:click={() => {
                const diff = i - currentIndex;
                rotation -= diff * angleIncrement;
                currentIndex = i;
              }}
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
    max-width: 550px;
    height: 450px;
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
    width: var(--card-width, 350px);
    height: 400px;
    left: 50%;
    top: 50%;
    margin-left: calc(var(--card-width, 350px) / -2);
    margin-top: -200px;
    transform-style: preserve-3d;
    transition:
      opacity 0.4s ease,
      transform 0.6s cubic-bezier(0.4, 0, 0.2, 1),
      filter 0.4s ease;
    opacity: 0.3;
    filter: blur(5px) grayscale(40%);
    pointer-events: none; /* Prevent clicking background cards */
  }

  .carousel-item.active {
    opacity: 1;
    filter: none;
    z-index: 100;
    pointer-events: auto;
  }

  /* Make the active card more solid for readability */
  .carousel-item.active :global(.card) {
    background: rgba(26, 26, 36, 0.95);
    border-color: var(--accent-primary);
    box-shadow: 0 0 40px rgba(0, 0, 0, 0.6);
  }

  .carousel-item :global(.card) {
    height: 100%;
    width: 100%;
  }

  /* Edit container - centered card */
  .edit-container {
    width: var(--card-width, 550px);
    height: auto;
    min-height: 450px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .edit-container :global(.card) {
    width: 100%;
    height: 100%;
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
    left: 2rem;
  }

  .nav-arrow-right {
    right: 2rem;
  }

  /* Indicators */
  .carousel-indicators {
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
