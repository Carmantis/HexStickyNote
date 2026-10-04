/**
 * Settings Store - the Ollama model the assistant uses
 *
 * Handles:
 * - Models installed in the local Ollama
 * - The selected model
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Types
// ============================================================================

export interface LocalModel {
  /** "ollama:<name>" */
  id: string;
  name: string;
  size: number | null;
  details: string | null;
  /** Whether the assistant can use it (tool calling) */
  supports_tools: boolean;
}

interface LocalModelList {
  models: LocalModel[];
  ollama_available: boolean;
}

interface SettingsState {
  models: LocalModel[];
  ollamaAvailable: boolean;
  activeModelId: string | null;
  isLoading: boolean;
  error: string | null;
}

// ============================================================================
// Store Implementation
// ============================================================================

function createSettingsStore() {
  const { subscribe, update } = writable<SettingsState>({
    models: [],
    ollamaAvailable: false,
    activeModelId: null,
    isLoading: false,
    error: null
  });

  return {
    subscribe,

    /**
     * Load available models and the selected model
     */
    async loadModels() {
      update(s => ({ ...s, isLoading: true, error: null }));

      try {
        const [list, activeModelId] = await Promise.all([
          invoke<LocalModelList>('list_local_models'),
          invoke<string | null>('get_active_model')
        ]);

        update(s => ({
          ...s,
          models: list.models,
          ollamaAvailable: list.ollama_available,
          activeModelId,
          isLoading: false
        }));
      } catch (error) {
        update(s => ({
          ...s,
          isLoading: false,
          error: error instanceof Error ? error.message : String(error)
        }));
      }
    },

    /**
     * Select the model the assistant uses (null clears the selection)
     */
    async setActiveModel(modelId: string | null) {
      try {
        await invoke('set_active_model', { modelId });
        update(s => ({ ...s, activeModelId: modelId, error: null }));
      } catch (error) {
        update(s => ({
          ...s,
          error: error instanceof Error ? error.message : String(error)
        }));
      }
    }
  };
}

// Export singleton store instance
export const settingsStore = createSettingsStore();

// ============================================================================
// Derived Stores
// ============================================================================

/**
 * The selected model, if it is currently available
 */
export const activeModel = derived(settingsStore, $store =>
  $store.activeModelId
    ? $store.models.find(m => m.id === $store.activeModelId) ?? null
    : null
);

/**
 * Format a byte count for display, e.g. "4.6 GB"
 */
export function formatBytes(bytes: number | null | undefined): string {
  if (!bytes) return '';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(unit >= 3 ? 1 : 0)} ${units[unit]}`;
}
