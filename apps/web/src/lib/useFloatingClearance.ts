/**
 * Keeps what floats in the bottom right corner (the approval cards and the
 * notices) clear of a view's bottom action bar. A bar registers how far its
 * top sits from the window's bottom edge; the floating layers sit that far up
 * plus a gap, instead of over the bar's buttons.
 */
import { useId, useLayoutEffect, useRef, type CSSProperties } from 'react';
import { create } from 'zustand';

/** The gap between a bar's top edge and what floats above it. */
export const FLOATING_GAP = 16;

/** Where the floating layers sit with no bar: just above the status bar (`bottom-11`). */
export const FLOATING_BOTTOM = 44;

/** How far the floating column reaches from the window's top edge at most. */
const FLOATING_TOP = 196;

interface ClearanceStore {
  /** Each mounted bar's distance from its top edge to the window's bottom edge, in px. */
  bars: Record<string, number>;
  set: (id: string, px: number | null) => void;
}

export const useClearanceStore = create<ClearanceStore>(set => ({
  bars: {},
  set: (id, px) =>
    set(s => {
      if (px === null) {
        if (!(id in s.bars)) return s;
        return { bars: Object.fromEntries(Object.entries(s.bars).filter(([k]) => k !== id)) };
      }
      return s.bars[id] === px ? s : { bars: { ...s.bars, [id]: px } };
    }),
}));

/** How far up from the window's bottom edge the floating layers sit. */
export function floatingBottom(bars: Record<string, number>): number {
  const highest = Math.max(0, ...Object.values(bars));
  return highest ? Math.max(FLOATING_BOTTOM, highest + FLOATING_GAP) : FLOATING_BOTTOM;
}

/**
 * The floating layers' lift as an inline style, or undefined with no bar on
 * show (they keep their `bottom-11`). With `column`, the height cap shrinks
 * by as much, so a tall stack still stops where it always did.
 */
export function useFloatingLift(column = false): CSSProperties | undefined {
  const bottom = useClearanceStore(s => floatingBottom(s.bars));
  if (bottom === FLOATING_BOTTOM) return undefined;
  return column ? { bottom, maxHeight: `calc(100vh - ${FLOATING_TOP + bottom}px)` } : { bottom };
}

/**
 * Marks a bottom action bar: the floating layers stay above it while it is
 * mounted. Attach the ref to the bar.
 */
export function useKeepClearOfFloating<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const id = useId();
  useLayoutEffect(() => {
    const el = ref.current;
    const { set } = useClearanceStore.getState();
    if (!el) return;
    const measure = () => {
      const rect = el.getBoundingClientRect();
      // No layout (tests) has no height, and nothing to keep clear.
      set(id, rect.height ? Math.max(0, Math.round(window.innerHeight - rect.top)) : null);
    };
    measure();
    window.addEventListener('resize', measure);
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    observer?.observe(el);
    return () => {
      window.removeEventListener('resize', measure);
      observer?.disconnect();
      set(id, null);
    };
  }, [id]);
  return ref;
}
