import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AiDigest } from '../types';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let _streamBuffer = $state('');
let _streaming = $state(false);
let _streamError = $state<string | null>(null);

let _weekDigest = $state<AiDigest | null>(null);
let _dayBriefing = $state<string | null>(null);
let _loadingDigest = $state(false);

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

export const stream = {
  get buffer() { return _streamBuffer; },
  get active() { return _streaming; },
  get error() { return _streamError; },
};

export const digest = {
  get week() { return _weekDigest; },
  get day() { return _dayBriefing; },
  get loading() { return _loadingDigest; },
};

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

let _tokenUnlisten: UnlistenFn | null = null;
let _doneUnlisten: UnlistenFn | null = null;

export async function startStream(prompt: string): Promise<void> {
  _streamBuffer = '';
  _streaming = true;
  _streamError = null;

  // Set up listeners before invoking so no tokens are missed
  _tokenUnlisten = await listen<string>('ai_token', (event) => {
    _streamBuffer += event.payload;
  });

  _doneUnlisten = await listen<void>('ai_done', () => {
    _streaming = false;
    cleanupListeners();
  });

  try {
    await invoke<void>('stream_ai_response', { prompt });
  } catch (e) {
    _streamError = String(e);
    _streaming = false;
    cleanupListeners();
  }
}

export function stopStream(): void {
  _streaming = false;
  cleanupListeners();
}

function cleanupListeners(): void {
  _tokenUnlisten?.();
  _doneUnlisten?.();
  _tokenUnlisten = null;
  _doneUnlisten = null;
}

// ---------------------------------------------------------------------------
// Day briefing
// ---------------------------------------------------------------------------

export async function loadDayBriefing(date: string): Promise<void> {
  _loadingDigest = true;
  _streamError = null;
  try {
    _dayBriefing = await invoke<string>('generate_day_briefing', { date });
  } catch (e) {
    _streamError = String(e);
  } finally {
    _loadingDigest = false;
  }
}

export async function refreshDayBriefing(date: string): Promise<void> {
  try {
    await invoke<void>('invalidate_day_digest', { date });
  } catch {
    // Ignore — cache may already be empty
  }
  await loadDayBriefing(date);
}

// ---------------------------------------------------------------------------
// Weekly digest
// ---------------------------------------------------------------------------

export async function loadWeekDigest(weekKey: string, anchorDate: string): Promise<void> {
  // Try cache first
  try {
    const cached = await invoke<AiDigest | null>('get_cached_digest', {
      periodType: 'week',
      periodKey: weekKey,
    });

    if (cached) {
      const ageMs = Date.now() - cached.created_at;
      if (ageMs < 24 * 60 * 60 * 1000) {
        _weekDigest = cached;
        return;
      }
    }
  } catch {
    // Cache miss or error — proceed to generate
  }

  _loadingDigest = true;
  _streamError = null;
  try {
    const content = await invoke<string>('generate_week_summary', {
      weekKey: weekKey,
      anchorDate: anchorDate,
    });
    _weekDigest = {
      id: '',
      period_type: 'week',
      period_key: weekKey,
      content,
      model: null,
      created_at: Date.now(),
    };
  } catch (e) {
    _streamError = String(e);
  } finally {
    _loadingDigest = false;
  }
}

// ---------------------------------------------------------------------------
// Reminder toast
// ---------------------------------------------------------------------------

let _activeToast = $state<{ title: string; body: string } | null>(null);

export const toast = {
  get active() { return _activeToast; },
};

export function showToast(title: string, body: string): void {
  _activeToast = { title, body };
}

export function dismissToast(): void {
  _activeToast = null;
}
