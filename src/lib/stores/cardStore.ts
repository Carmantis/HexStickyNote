/**
 * Card Store - Manages card state and view/edit mode transitions
 *
 * This store tracks:
 * - All cards in the workspace
 * - Which card is currently being edited (if any)
 * - Card content updates from the editor and the assistant
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
  /** Unix timestamp in seconds (as stored by the backend) */
  created_at: number;
  /** Unix timestamp in seconds (as stored by the backend) */
  updated_at: number;
  /** Folder (subdirectory) the note is stored in; "" for none */
  folder: string;
}

/** `null` shows the notes of every folder */
export type FolderFilter = string | null;

/** The notes outside any folder */
export const NO_FOLDER = '';

const FOLDER_KEY = 'hexstickynote.folder';
const LAYOUT_KEY = 'hexstickynote.notesLayout';

/** How the notes are shown: the 3D carousel or a grid of small tiles */
export type NotesLayout = 'carousel' | 'grid';

function loadLayout(): NotesLayout {
  try {
    return localStorage.getItem(LAYOUT_KEY) === 'grid' ? 'grid' : 'carousel';
  } catch {
    return 'carousel';
  }
}

function loadFolderFilter(): FolderFilter {
  try {
    return localStorage.getItem(FOLDER_KEY);
  } catch {
    return null;
  }
}

function saveFolderFilter(folder: FolderFilter) {
  try {
    if (folder === null) localStorage.removeItem(FOLDER_KEY);
    else localStorage.setItem(FOLDER_KEY, folder);
  } catch {
    // Not remembered; the filter still applies
  }
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

export type CardMode = 'view' | 'edit';

interface CardState {
  cards: Card[];
  /** Folder names, including empty folders */
  folders: string[];
  /** Folder whose notes the carousel shows */
  folder: FolderFilter;
  layout: NotesLayout;
  /** Card the carousel should bring to the front (picked from the grid) */
  focusCardId: string | null;
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
    folders: [],
    folder: typeof window !== 'undefined' ? loadFolderFilter() : null,
    layout: typeof window !== 'undefined' ? loadLayout() : 'carousel',
    focusCardId: null,
    editingCardId: null,
    isLoading: false,
    error: null
  });

  /** Set the folder list; a filter on a folder that is gone falls back to all notes */
  function withFolders(s: CardState, folders: string[]): CardState {
    const folder = s.folder && !folders.includes(s.folder) ? null : s.folder;
    if (folder !== s.folder) saveFolderFilter(folder);
    return { ...s, folders, folder };
  }

  async function refreshFolders() {
    const folders = await invoke<string[]>('list_folders');
    update(s => withFolders(s, folders));
  }

  /** Run a folder command, reloading the notes afterwards when they moved */
  async function folderAction<T>(action: () => Promise<T>, reload: boolean): Promise<T | null> {
    try {
      const result = await action();
      if (reload) await reloadCards(true);
      else await refreshFolders();
      return result;
    } catch (error) {
      update(s => ({ ...s, error: errorText(error) }));
      return null;
    }
  }

  /**
   * Load all cards from the backend
   */
  async function loadCards() {
    update(s => ({ ...s, isLoading: true, error: null }));

    try {
      const [cards, folders] = await Promise.all([
        invoke<Card[]>('get_cards'),
        invoke<string[]>('list_folders')
      ]);
      update(s => withFolders({ ...s, cards, isLoading: false }, folders));
    } catch (error) {
      update(s => ({
        ...s,
        isLoading: false,
        error: error instanceof Error ? error.message : String(error)
      }));
    }
  }

  /**
   * Reload all cards from the file system
   * Useful when cards are modified externally (e.g., by Claude Desktop)
   * @param quiet Keep the carousel (and an open editor) on screen while loading
   */
  async function reloadCards(quiet = false) {
    update(s => ({ ...s, isLoading: !quiet, error: null }));

    try {
      const cards = await invoke<Card[]>('reload_cards');
      const folders = await invoke<string[]>('list_folders');
      update(s => withFolders({ ...s, cards, isLoading: false }, folders));
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
      reloadCards();
    });
  }

  /**
   * Write the card being edited to disk (e.g. before the assistant reads or changes it)
   */
  async function saveEditingCard() {
    let editing: Card | undefined;
    update(s => {
      editing = s.cards.find(c => c.id === s.editingCardId);
      return s;
    });
    if (editing) {
      await invoke('save_card', { card: editing });
    }
  }

  function setLayout(layout: NotesLayout) {
    try {
      localStorage.setItem(LAYOUT_KEY, layout);
    } catch {
      // Not remembered; the layout still changes
    }
    update(s => ({ ...s, layout }));
  }

  return {
    subscribe,
    loadCards,
    reloadCards,
    saveEditingCard,

    /**
     * Create a new card
     */
    async createCard(content: string = ''): Promise<Card | null> {
      let folder: FolderFilter = null;
      update(s => {
        folder = s.folder;
        return s;
      });
      try {
        // New notes go into the folder being viewed
        const card = await invoke<Card>('create_card', { content, folder });
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
     * Update card content (used during editing)
     */
    updateCardContent(cardId: string, content: string) {
      update(s => ({
        ...s,
        cards: s.cards.map(card =>
          card.id === cardId
            ? { ...card, content, updated_at: nowSeconds() }
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

    /** Show the notes of one folder (`NO_FOLDER` for the ones outside folders) or all (`null`) */
    selectFolder(folder: FolderFilter) {
      saveFolderFilter(folder);
      update(s => ({ ...s, folder }));
    },

    async createFolder(name: string): Promise<string | null> {
      return folderAction(() => invoke<string>('create_folder', { name }), false);
    },

    async renameFolder(from: string, to: string): Promise<string | null> {
      const name = await folderAction(() => invoke<string>('rename_folder', { from, to }), true);
      if (name !== null) {
        update(s => {
          if (s.folder !== from) return s;
          saveFolderFilter(name);
          return { ...s, folder: name };
        });
      }
      return name;
    },

    /** Delete a folder; its notes move out of it and are kept */
    async deleteFolder(name: string): Promise<boolean> {
      return (await folderAction(() => invoke('delete_folder', { name }), true)) !== null;
    },

    /** Move a note to another folder (`NO_FOLDER` for none) */
    async moveCard(cardId: string, folder: string): Promise<boolean> {
      let content: string | undefined;
      update(s => {
        content = s.cards.find(c => c.id === cardId)?.content;
        return s;
      });
      try {
        // Unsaved edits travel with the note
        if (content !== undefined) {
          await invoke('save_card', { card: { id: cardId, content, created_at: 0, updated_at: 0, folder } });
        }
        const moved = await invoke<Card>('move_card', { id: cardId, folder });
        update(s => ({ ...s, cards: s.cards.map(c => (c.id === cardId ? moved : c)) }));
        await refreshFolders();
        return true;
      } catch (error) {
        update(s => ({ ...s, error: errorText(error) }));
        return false;
      }
    },

    setLayout,

    toggleLayout() {
      let layout: NotesLayout = 'carousel';
      update(s => {
        layout = s.layout === 'grid' ? 'carousel' : 'grid';
        return s;
      });
      setLayout(layout);
    },

    /** Go back to the carousel with this card in front */
    showInCarousel(cardId: string) {
      update(s => ({ ...s, focusCardId: cardId }));
      setLayout('carousel');
    },

    /** The carousel has brought the focused card to the front */
    clearFocus() {
      update(s => ({ ...s, focusCardId: null }));
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
 * The notes of the selected folder
 */
export const visibleCards = derived(cardStore, $store =>
  $store.folder === null ? $store.cards : $store.cards.filter(c => c.folder === $store.folder)
);

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
