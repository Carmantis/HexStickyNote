/**
 * Settings Store - Manages AI provider configuration
 *
 * Handles:
 * - Provider selection
 * - API key status (configured/not configured)
 * - AI streaming state
 */

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ============================================================================
// Types
// ============================================================================

export interface Provider {
  id: string;
  name: string;
  configured: boolean;
}

export interface AiStreamChunk {
  chunk: string;
  done: boolean;
  gpu_info?: string;
}

interface SettingsState {
  providers: Provider[];
  activeProviderId: string | null;
  isLoading: boolean;
  isStreaming: boolean;
  error: string | null;
  currentGpuInfo: string | null;
}

// ============================================================================
// Store Implementation
// ============================================================================

function createSettingsStore() {
  const { subscribe, set, update } = writable<SettingsState>({
    providers: [],
    activeProviderId: null,
    isLoading: false,
    isStreaming: false,
    error: null,
    currentGpuInfo: null
  });

  let streamUnlisten: UnlistenFn | null = null;

  return {
    subscribe,

    /**
     * Load providers and their configuration status
     */
    async loadProviders() {
      update(s => ({ ...s, isLoading: true, error: null }));

      try {
        const providers = await invoke<Provider[]>('get_providers');
        const activeProvider = await invoke<string | null>('get_active_provider');

        update(s => ({
          ...s,
          providers,
          activeProviderId: activeProvider,
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
     * Save an API key for a provider
     */
    async saveApiKey(providerId: string, apiKey: string) {
      update(s => ({ ...s, isLoading: true, error: null }));

      try {
        await invoke('save_api_key', { provider: providerId, key: apiKey });

        // Refresh providers to update configured status
        await this.loadProviders();
      } catch (error) {
        update(s => ({
          ...s,
          isLoading: false,
          error: error instanceof Error ? error.message : String(error)
        }));
      }
    },

    /**
     * Delete an API key for a provider
     */
    async deleteApiKey(providerId: string) {
      try {
        await invoke('delete_api_key', { provider: providerId });
        await this.loadProviders();
      } catch (error) {
        update(s => ({
          ...s,
          error: error instanceof Error ? error.message : String(error)
        }));
      }
    },

    /**
     * Set the active AI provider
     */
    async setActiveProvider(providerId: string) {
      try {
        await invoke('set_active_provider', { provider: providerId });
        update(s => ({ ...s, activeProviderId: providerId }));
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
 * Get the currently active provider object
 */
export const activeProvider = derived(settingsStore, $store =>
  $store.activeProviderId
    ? $store.providers.find(p => p.id === $store.activeProviderId) ?? null
    : null
);

/**
 * Check if any provider is configured
 */
export const hasConfiguredProvider = derived(settingsStore, $store =>
  $store.providers.some(p => p.configured)
);

/**
 * Check if AI is ready to use (has active provider with API key)
 */
export const isAiReady = derived(settingsStore, $store => {
  if (!$store.activeProviderId) return false;
  const provider = $store.providers.find(p => p.id === $store.activeProviderId);
  return provider?.configured ?? false;
});
