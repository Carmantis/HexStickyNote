// ---------------------------------------------------------------------------
// Gesture handler — platform-agnostic drag/swipe via pointer events.
// Use as a Svelte action: <div use:gestureAction={callbacks}>
// ---------------------------------------------------------------------------

export interface GestureCallbacks {
  onDragStart?: (offset: number) => void;
  onDragMove?: (offset: number) => void;
  onDragEnd?: (offset: number, velocity: number) => void;
}

export interface GestureAction {
  destroy(): void;
  update(newCallbacks: GestureCallbacks): void;
}

const SNAP_THRESHOLD_PX = 80;
const MIN_VELOCITY_PX_MS = 0.3; // px/ms to trigger snap without reaching threshold

/**
 * Svelte use: action for drag / swipe gestures.
 *
 * Example:
 *   <div use:gestureAction={{ onDragEnd: handleEnd }}>
 */
export function gestureAction(node: HTMLElement, callbacks: GestureCallbacks): GestureAction {
  let cb = callbacks;
  let startX = 0;
  let startTime = 0;
  let dragging = false;

  function onPointerDown(e: PointerEvent) {
    // Only respond to primary (left-click / single touch)
    if (e.button !== 0 && e.pointerType !== 'touch') return;

    // Don't capture clicks that originate on interactive elements —
    // setPointerCapture would redirect the click event away from the button.
    if ((e.target as Element).closest('button, a, input, textarea, [role="button"]')) return;

    dragging = true;
    startX = e.clientX;
    startTime = e.timeStamp;
    
    node.setPointerCapture(e.pointerId);
    cb.onDragStart?.(0);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const offset = e.clientX - startX;
    cb.onDragMove?.(offset);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;

    const offset = e.clientX - startX;
    const elapsed = Math.max(1, e.timeStamp - startTime);
    const velocity = offset / elapsed; // px/ms

    cb.onDragEnd?.(offset, velocity);
      }

  function onPointerCancel() {
    if (!dragging) return;
    dragging = false;
    cb.onDragEnd?.(0, 0);
      }

  node.addEventListener('pointerdown', onPointerDown);
  node.addEventListener('pointermove', onPointerMove);
  node.addEventListener('pointerup', onPointerUp);
  node.addEventListener('pointercancel', onPointerCancel);

  return {
    update(newCallbacks: GestureCallbacks) {
      cb = newCallbacks;
    },
    destroy() {
      node.removeEventListener('pointerdown', onPointerDown);
      node.removeEventListener('pointermove', onPointerMove);
      node.removeEventListener('pointerup', onPointerUp);
      node.removeEventListener('pointercancel', onPointerCancel);
    },
  };
}

/**
 * Determines whether a completed drag should trigger a navigation.
 * Returns -1 (go back), 1 (go forward), or 0 (snap back).
 */
export function resolveSwipe(offset: number, velocity: number): -1 | 0 | 1 {
  if (Math.abs(offset) >= SNAP_THRESHOLD_PX) {
    return offset < 0 ? 1 : -1;
  }
  if (Math.abs(velocity) >= MIN_VELOCITY_PX_MS) {
    return velocity < 0 ? 1 : -1;
  }
  return 0;
}
