/**
 * Assistant Store - the chat with the tool-using assistant
 *
 * The backend keeps no conversation state: the full message list (Ollama chat
 * format) lives here and goes along with every call. Changes the assistant
 * wants to make come back as pending actions that the user approves or
 * declines before anything is written.
 */

import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { cardStore } from './cardStore';

// ============================================================================
// Types
// ============================================================================

export interface PendingAction {
  call_id: string | null;
  tool: string;
  arguments: unknown;
  summary: string;
}

export interface ActionRecord {
  tool: string;
  summary: string;
  ok: boolean;
  error: string | null;
}

interface AssistantTurn {
  messages: unknown[];
  reply: string | null;
  pending: PendingAction[];
  actions: ActionRecord[];
}

/** What the user is looking at, so "this note" refers to the open note */
export interface AssistantContext {
  view: string;
  open_note_id: string | null;
}

export type ChatEntry =
  | { kind: 'user'; text: string }
  | { kind: 'assistant'; text: string }
  | { kind: 'action'; record: ActionRecord }
  | { kind: 'error'; text: string };

interface AssistantState {
  entries: ChatEntry[];
  /** Conversation in Ollama's format, sent back with every call */
  messages: unknown[];
  /** Actions waiting for the user's decision; index-aligned with `decisions` */
  pending: PendingAction[];
  decisions: (boolean | null)[];
  isBusy: boolean;
}

/** Tools whose results the other views need to reload */
const NOTE_TOOLS = ['create_note', 'update_note'];
const CALENDAR_TOOLS = ['create_event'];
const TIME_TOOLS = ['start_timer', 'stop_timer'];

/** HexTimeView listens for this to reload its page */
export const HEXTIME_CHANGED_EVENT = 'hextime-changed';

// ============================================================================
// Store Implementation
// ============================================================================

function createAssistantStore() {
  const initial: AssistantState = {
    entries: [],
    messages: [],
    pending: [],
    decisions: [],
    isBusy: false
  };
  const { subscribe, set, update } = writable<AssistantState>({ ...initial });

  async function refreshViews(actions: ActionRecord[]) {
    const changed = (tools: string[]) => actions.some(a => a.ok && tools.includes(a.tool));

    if (changed(NOTE_TOOLS)) {
      await cardStore.reloadCards(true);
    }
    if (changed(CALENDAR_TOOLS)) {
      const { refresh } = await import('$lib/calendar/stores/carousel.svelte');
      await refresh();
    }
    if (changed(TIME_TOOLS)) {
      window.dispatchEvent(new CustomEvent(HEXTIME_CHANGED_EVENT));
    }
  }

  function applyTurn(turn: AssistantTurn) {
    update(s => {
      const entries: ChatEntry[] = [...s.entries];
      for (const record of turn.actions) {
        entries.push({ kind: 'action', record });
      }
      if (turn.reply) {
        entries.push({ kind: 'assistant', text: turn.reply });
      }
      return {
        ...s,
        entries,
        messages: turn.messages,
        pending: turn.pending,
        decisions: turn.pending.map(() => null),
        isBusy: false
      };
    });
    void refreshViews(turn.actions);
  }

  function fail(error: unknown) {
    update(s => ({
      ...s,
      isBusy: false,
      entries: [...s.entries, { kind: 'error', text: String(error) }]
    }));
  }

  return {
    subscribe,

    /**
     * Send a user message
     */
    async send(text: string, context: AssistantContext) {
      const content = text.trim();
      const state = get({ subscribe });
      if (!content || state.isBusy || state.pending.length > 0) return;

      const messages = [...state.messages, { role: 'user', content }];
      update(s => ({
        ...s,
        isBusy: true,
        entries: [...s.entries, { kind: 'user', text: content }]
      }));

      try {
        // The assistant reads notes from disk; include unsaved edits
        await cardStore.saveEditingCard();
        applyTurn(await invoke<AssistantTurn>('assistant_send', { messages, context }));
      } catch (error) {
        fail(error);
      }
    },

    /**
     * Approve or decline one pending action; runs them once all are decided
     */
    async decide(index: number, approved: boolean, context: AssistantContext) {
      const state = get({ subscribe });
      if (state.isBusy || index >= state.pending.length) return;

      const decisions = [...state.decisions];
      decisions[index] = approved;
      const ready = decisions.every(d => d !== null);
      update(s => ({ ...s, decisions, isBusy: ready }));
      if (!ready) return;

      const request = {
        messages: state.messages,
        decisions: state.pending.map((action, i) => ({ ...action, approved: decisions[i] })),
        context
      };

      try {
        await cardStore.saveEditingCard();
        applyTurn(await invoke<AssistantTurn>('assistant_confirm', request));
      } catch (error) {
        update(s => ({ ...s, pending: [], decisions: [] }));
        fail(error);
      }
    },

    /**
     * Start a new conversation
     */
    reset() {
      set({ ...initial });
    }
  };
}

export const assistantStore = createAssistantStore();
