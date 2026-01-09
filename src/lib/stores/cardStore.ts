/**
 * Card Store - Manages card state and view/edit mode transitions
 *
 * This store tracks:
 * - All cards in the workspace
 * - Which card is currently being edited (if any)
 * - Card content updates from AI streaming
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ============================================================================
// Types
// ============================================================================

export interface Card {
  id: string;
  content: string;
  created_at: number;
  updated_at: number;
}

export type CardMode = 'view' | 'edit';

interface CardState {
  cards: Card[];
  editingCardId: string | null;
  isLoading: boolean;
  error: string | null;
}

// ============================================================================
// Store Implementation
// ============================================================================

function createCardStore() {
  const { subscribe, set, update } = writable<CardState>({
    cards: [],
    editingCardId: null,
    isLoading: false,
    error: null
  });

  /**
   * Load all cards from the backend
   */
  async function loadCards() {
    update(s => ({ ...s, isLoading: true, error: null }));

    try {
      const cards = await invoke<Card[]>('get_cards');
      update(s => ({ ...s, cards, isLoading: false }));
    } catch (error) {
      update(s => ({
        ...s,
        isLoading: false,
        error: error instanceof Error ? error.message : String(error)
      }));
    }
  }

  // Listen for backend refresh requests (e.g. after AI tool execution)
  if (typeof window !== 'undefined') {
    listen('refresh-required', () => {
      loadCards();
    });
  }

  return {
    subscribe,
    loadCards,

    /**
     * Create a new card
     */
    async createCard(content: string = ''): Promise<Card | null> {
      try {
        const card = await invoke<Card>('create_card', { content });
        update(s => ({ ...s, cards: [...s.cards, card] }));
        return card;
      } catch (error) {
        update(s => ({
          ...s,
          error: error instanceof Error ? error.message : String(error)
        }));
        return null;
      }
    },

    /**
     * Enter edit mode for a specific card
     */
    enterEditMode(cardId: string) {
      update(s => {
        // If we were editing another card, save it first
        if (s.editingCardId && s.editingCardId !== cardId) {
          const prevCard = s.cards.find(c => c.id === s.editingCardId);
          if (prevCard) {
            invoke('save_card', { card: prevCard }).catch(console.error);
          }
        }
        return { ...s, editingCardId: cardId };
      });
    },

    /**
     * Exit edit mode and save the card
     * @param requestingCardId Optional ID of the card requesting to exit.
     *                         If provided, exit only if it matches the current editing card.
     */
    async exitEditMode(requestingCardId?: string) {
      update(s => {
        if (!s.editingCardId) return s;
        
        // Prevent race condition: if we switched cards (enterEditMode for B ran),
        // but the previous card (A) tries to exit (via clickOutside), ignore A's request.
        if (requestingCardId && s.editingCardId !== requestingCardId) {
          return s;
        }

        // Save to backend
        const editedCard = s.cards.find(c => c.id === s.editingCardId);
        if (editedCard) {
          invoke('save_card', { card: editedCard }).catch(console.error);
        }

        return { ...s, editingCardId: null };
      });
    },

    /**
     * Update card content (used during editing or AI streaming)
     */
    updateCardContent(cardId: string, content: string) {
      update(s => ({
        ...s,
        cards: s.cards.map(card =>
          card.id === cardId
            ? { ...card, content, updated_at: Date.now() }
            : card
        )
      }));
    },

    /**
     * Append content to a card (used for AI streaming)
     */
    appendToCard(cardId: string, chunk: string) {
      update(s => ({
        ...s,
        cards: s.cards.map(card =>
          card.id === cardId
            ? { ...card, content: card.content + chunk, updated_at: Date.now() }
            : card
        )
      }));
    },

    /**
     * Delete a card
     */
    async deleteCard(cardId: string) {
      try {
        await invoke('delete_card', { id: cardId });
        update(s => ({
          ...s,
          cards: s.cards.filter(c => c.id !== cardId),
          editingCardId: s.editingCardId === cardId ? null : s.editingCardId
        }));
      } catch (error) {
        update(s => ({
          ...s,
          error: error instanceof Error ? error.message : String(error)
        }));
      }
    },

    /**
     * Clear any errors
     */
    clearError() {
      update(s => ({ ...s, error: null }));
    }
  };
}

// Export singleton store instance
export const cardStore = createCardStore();

// ============================================================================
// Derived Stores
// ============================================================================

/**
 * Get the currently edited card (if any)
 */
export const editingCard = derived(cardStore, $store =>
  $store.editingCardId
    ? $store.cards.find(c => c.id === $store.editingCardId) ?? null
    : null
);

/**
 * Check if a specific card is in edit mode
 */
export function isCardEditing(cardId: string) {
  return derived(cardStore, $store => $store.editingCardId === cardId);
}

/**
 * Get the mode for a specific card
 */
export function getCardMode(cardId: string) {
  return derived(cardStore, $store =>
    $store.editingCardId === cardId ? 'edit' : 'view'
  );
}
