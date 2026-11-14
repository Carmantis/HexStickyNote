// ---------------------------------------------------------------------------
// Reminder toast — shown when the Rust scheduler emits `reminder_due`
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
