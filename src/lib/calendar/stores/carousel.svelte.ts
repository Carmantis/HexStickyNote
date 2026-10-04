import { invoke } from '@tauri-apps/api/core';
import { resolveSwipe } from '../utils/gestures';
import { offsetAnchor, todayIso } from '../utils/dateHelpers';
import type { CalendarData, CarouselSlide, DayNotes, ViewMode } from '../types';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let _position = $state(0); // -1 = prev, 0 = current, 1 = next
let _viewMode = $state<ViewMode>('month');
let _isDragging = $state(false);
let _dragOffset = $state(0);
let _transitioning = $state(false);

// Three slides: [prev, current, next]
let _slides = $state<CarouselSlide[]>([
  { anchor_date: '', data: null, loading: false },
  { anchor_date: todayIso(), data: null, loading: false },
  { anchor_date: '', data: null, loading: false },
]);

// ---------------------------------------------------------------------------
// Derived
// ---------------------------------------------------------------------------

// The track is 300% wide (3 slides × 100% viewport). Each slide step = 100%/3 of the track.
// Slide indices: 0 = prev, 1 = current, 2 = next.
// _position: -1 = animating to prev, 0 = resting on current, 1 = animating to next.
// Mapping: visible slide index = _position + 1.
export const transform = { get current() { return `translateX(calc(${-(_position + 1) * (100 / 3)}% + ${_dragOffset}px))`; } };

export const viewMode = { get current() { return _viewMode; } };
export const isDragging = { get current() { return _isDragging; } };
export const dragOffset = { get current() { return _dragOffset; } };
export const transitioning = { get current() { return _transitioning; } };
export const slides = { get current() { return _slides; } };

export const currentSlide = { get current() { return _slides[1]; } };
export const currentAnchor = { get current() { return _slides[1].anchor_date; } };

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

export async function initialise(mode?: ViewMode): Promise<void> {
  if (mode) _viewMode = mode;
  await prefetchAll(_slides[1].anchor_date);
}

export async function navigateNext(): Promise<void> {
  if (_transitioning) return;
  await slide(1);
}

export async function navigatePrev(): Promise<void> {
  if (_transitioning) return;
  await slide(-1);
}

async function slide(direction: -1 | 1): Promise<void> {
  _transitioning = true;
  _position = direction;

  // After CSS transition completes (300ms), reshuffle slides
  await sleep(310);

  const newCurrentAnchor = direction === 1
    ? _slides[2].anchor_date
    : _slides[0].anchor_date;

  reshuffleSlides(direction, newCurrentAnchor);
  _position = 0;
  _transitioning = false;

  // Prefetch new neighbours
  await prefetchNeighbours(newCurrentAnchor);
}

function reshuffleSlides(direction: -1 | 1, newAnchor: string): void {
  if (direction === 1) {
    // Moved forward: prev = old current, current = old next, next = new placeholder
    _slides = [
      _slides[1],
      _slides[2],
      {
        anchor_date: offsetAnchor(newAnchor, 1, _viewMode),
        data: null,
        loading: false,
      },
    ];
  } else {
    // Moved back: prev = new placeholder, current = old prev, next = old current
    _slides = [
      {
        anchor_date: offsetAnchor(newAnchor, -1, _viewMode),
        data: null,
        loading: false,
      },
      _slides[0],
      _slides[1],
    ];
  }
}

// ---------------------------------------------------------------------------
// View switching
// ---------------------------------------------------------------------------

export async function switchView(mode: ViewMode): Promise<void> {
  _viewMode = mode;
  const anchor = _slides[1].anchor_date;
  // Reset all slides
  _slides = [
    { anchor_date: offsetAnchor(anchor, -1, mode), data: null, loading: false },
    { anchor_date: anchor, data: null, loading: false },
    { anchor_date: offsetAnchor(anchor, 1, mode), data: null, loading: false },
  ];
  await prefetchAll(anchor);
}

export async function jumpToDate(isoDate: string): Promise<void> {
  _slides = [
    { anchor_date: offsetAnchor(isoDate, -1, _viewMode), data: null, loading: false },
    { anchor_date: isoDate, data: null, loading: false },
    { anchor_date: offsetAnchor(isoDate, 1, _viewMode), data: null, loading: false },
  ];
  await prefetchAll(isoDate);
}

// ---------------------------------------------------------------------------
// Drag gesture handlers (called by CarouselEngine)
// ---------------------------------------------------------------------------

export function onDragStart(_offset: number): void {
  _isDragging = true;
}

export function onDragMove(offset: number): void {
  _dragOffset = offset;
}

export function onDragEnd(offset: number, velocity: number): void {
  _isDragging = false;
  _dragOffset = 0;

  const direction = resolveSwipe(offset, velocity);
  if (direction !== 0) {
    slide(direction);
  }
}

// ---------------------------------------------------------------------------
// Data fetching
// ---------------------------------------------------------------------------

async function fetchSlideData(anchor: string): Promise<CalendarData> {
  return invoke<CalendarData>('get_calendar_data', {
    mode: _viewMode,
    anchorDate: anchor,
  });
}

async function prefetchAll(anchor: string): Promise<void> {
  const anchors = [
    offsetAnchor(anchor, -1, _viewMode),
    anchor,
    offsetAnchor(anchor, 1, _viewMode),
  ];

  _slides = anchors.map((a) => ({ anchor_date: a, data: null, loading: true }));

  const results = await Promise.allSettled(anchors.map(fetchSlideData));

  _slides = anchors.map((a, i) => {
    const result = results[i];
    if (result.status === 'rejected') {
      console.error(`[carousel] Failed to load slide for ${a}:`, result.reason);
    }
    return {
      anchor_date: a,
      data: result.status === 'fulfilled' ? result.value : null,
      loading: false,
    };
  });
}

/**
 * Re-fetch the visible slides in place (no loading state), e.g. after an
 * event was created, edited or deleted.
 */
export async function refresh(): Promise<void> {
  const anchors = _slides.map((slide) => slide.anchor_date);
  const results = await Promise.allSettled(anchors.map(fetchSlideData));

  // The user may have navigated while the requests were in flight
  if (_slides.some((slide, i) => slide.anchor_date !== anchors[i])) return;

  _slides = _slides.map((slide, i) => {
    const result = results[i];
    return result.status === 'fulfilled' ? { ...slide, data: result.value, loading: false } : slide;
  });
}

async function prefetchNeighbours(_anchor: string): Promise<void> {
  const prevAnchor = _slides[0].anchor_date;
  const nextAnchor = _slides[2].anchor_date;

  // Mark loading
  _slides = [
    { ..._slides[0], loading: !_slides[0].data },
    _slides[1],
    { ..._slides[2], loading: !_slides[2].data },
  ];

  const [prevResult, nextResult] = await Promise.allSettled([
    _slides[0].data ? Promise.resolve(_slides[0].data) : fetchSlideData(prevAnchor),
    _slides[2].data ? Promise.resolve(_slides[2].data) : fetchSlideData(nextAnchor),
  ]);

  _slides = [
    {
      anchor_date: prevAnchor,
      data: prevResult.status === 'fulfilled' ? prevResult.value : null,
      loading: false,
    },
    _slides[1],
    {
      anchor_date: nextAnchor,
      data: nextResult.status === 'fulfilled' ? nextResult.value : null,
      loading: false,
    },
  ];
}

// ---------------------------------------------------------------------------
// Live patch helpers
// ---------------------------------------------------------------------------

/** Update a single note entry in all slide caches that contain that date. */
export function patchNote(note: DayNotes): void {
  _slides = _slides.map((slide) => {
    if (!slide.data) return slide;
    const existing = slide.data.notes.some((n) => n.date === note.date);
    const notes = existing
      ? slide.data.notes.map((n) => (n.date === note.date ? note : n))
      : [...slide.data.notes, note];
    return { ...slide, data: { ...slide.data, notes } };
  });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
