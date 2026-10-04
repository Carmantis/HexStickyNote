/**
 * Settings Store - Manages local AI model selection
 *
 * Handles:
 * - Models downloaded by the app and models installed in Ollama
 * - The selected model
 * - AI streaming state
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ============================================================================
// Types
// ============================================================================

export interface LocalModel {
  /** "app:<file>.gguf" or "ollama:<name>" */
  id: string;
  name: string;
  source: 'app' | 'ollama';
  size: number | null;
  details: string | null;
}

interface LocalModelList {
  models: LocalModel[];
  ollama_available: boolean;
}

export interface AiStreamChunk {
  chunk: string;
  done: boolean;
  gpu_info?: string;
}

interface SettingsState {
  models: LocalModel[];
  ollamaAvailable: boolean;
  activeModelId: string | null;
  isLoading: boolean;
  isStreaming: boolean;
  error: string | null;
  currentGpuInfo: string | null;
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
    isStreaming: false,
    error: null,
    currentGpuInfo: null
  });

  let streamUnlisten: UnlistenFn | null = null;

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
     * Select the model used for AI writing (null clears the selection)
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
    },

    /**
     * Invoke AI with streaming response
     * Returns a function to stop listening
     */
    async invokeAiStream(
      prompt: string,
      context: string,
      onChunk: (chunk: string) => void,
      onDone: () => void,
      onError: (error: string) => void
    ) {
      // Clean up previous listener
      if (streamUnlisten) {
        streamUnlisten();
        streamUnlisten = null;
      }

      update(s => ({ ...s, isStreaming: true, error: null, currentGpuInfo: null }));

      try {
        // Set up event listener for streaming chunks
        streamUnlisten = await listen<AiStreamChunk>('ai-stream-chunk', (event) => {
          if (event.payload.gpu_info) {
            update(s => ({ ...s, currentGpuInfo: event.payload.gpu_info || null }));
          }

          if (event.payload.done) {
            update(s => ({ ...s, isStreaming: false }));
            onDone();

            if (streamUnlisten) {
              streamUnlisten();
              streamUnlisten = null;
            }
          } else {
            onChunk(event.payload.chunk);
          }
        });

        // Start the stream
        await invoke('invoke_ai_stream', { prompt, context });
      } catch (error) {
        update(s => ({
          ...s,
          isStreaming: false,
          error: error instanceof Error ? error.message : String(error)
        }));
        onError(error instanceof Error ? error.message : String(error));

        if (streamUnlisten) {
          streamUnlisten();
          streamUnlisten = null;
        }
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
 * Check if AI is ready to use (the selected model is available)
 */
export const isAiReady = derived(activeModel, $model => $model !== null);

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
