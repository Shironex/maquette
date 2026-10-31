import { createElement, lazy, type ComponentType, type ReactNode } from 'react';

/** A hidden window runs no frames, so this much later the work goes ahead anyway. */
const NO_FRAME_MS = 100;

/**
 * Resolves after the browser has had a frame to paint what was just committed.
 * Work that blocks the main thread waits for this, or its loader never shows.
 */
export function afterPaint(): Promise<void> {
  return new Promise(resolve => {
    const timer = setTimeout(resolve, NO_FRAME_MS);
    requestAnimationFrame(() => {
      clearTimeout(timer);
      setTimeout(resolve, 0);
    });
  });
}

export type Preloadable<P> = ((props: P) => ReactNode) & {
  /** Fetches the code now; once it is here the component renders with no fallback at all. */
  preload: () => Promise<unknown>;
};

/**
 * `lazy`, but the module is fetched and evaluated only once the fallback has
 * painted, and code that was preloaded renders at once: React holds a fallback
 * for 300 ms, so even a chunk already in memory would otherwise show one.
 */
export function lazyAfterPaint<P extends object>(
  load: () => Promise<{ default: (props: P) => ReactNode }>
): Preloadable<P> {
  let preloaded: ((props: P) => ReactNode) | null = null;
  // Once `lazy` has been asked it stays: swapping the type would remount the view.
  let lazyAsked = false;
  const later = lazy(() => {
    lazyAsked = true;
    return afterPaint().then(load);
  }) as unknown as ComponentType<P>;
  const preload = () =>
    load().then(module => {
      if (!lazyAsked) preloaded = module.default;
    });
  const show = (props: P) => createElement((preloaded ?? later) as ComponentType<P>, props);
  return Object.assign(show, { preload });
}
