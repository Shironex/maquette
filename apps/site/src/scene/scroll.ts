/**
 * Tracks where the page is in the tour, for the page's copy and timeline and
 * for the island's camera. The tour's layout is measured once and again on
 * resize, late fonts and load, so a scroll is arithmetic on scrollY with no
 * layout reads. No three.js imports.
 */
import { LAST_STOP, progressAt, scrollFor } from './progress';
import type { TourScroll } from './tour';

export interface TourTracker extends TourScroll {
  /** Scrolls the page to a stop. */
  scrollToStop(stop: number, smooth: boolean): void;
}

/**
 * Starts tracking `tour`, whose sticky `stage` stays on screen while it
 * scrolls. While `active` does not match (narrow windows) progress stays 0.
 */
export function trackTour(
  tour: HTMLElement,
  stage: HTMLElement,
  active: MediaQueryList
): TourTracker {
  const listeners = new Set<(progress: number) => void>();
  let top = 0;
  let span = 0;
  let progress = 0;

  const update = () => {
    const next = active.matches ? progressAt(window.scrollY, top, span, LAST_STOP) : 0;
    if (next === progress) return;
    progress = next;
    for (const listener of listeners) listener(progress);
  };
  const measure = () => {
    top = tour.getBoundingClientRect().top + window.scrollY;
    span = tour.offsetHeight - stage.offsetHeight;
    update();
  };

  measure();
  window.addEventListener('scroll', update, { passive: true });
  window.addEventListener('resize', measure, { passive: true });
  active.addEventListener('change', measure);
  new ResizeObserver(measure).observe(tour);
  // Late fonts and images move the tour without always resizing it.
  void document.fonts?.ready.then(measure);
  if (document.readyState !== 'complete') window.addEventListener('load', measure, { once: true });

  return {
    read: () => progress,
    subscribe(listener) {
      listeners.add(listener);
      listener(progress);
      return () => listeners.delete(listener);
    },
    scrollToStop(stop, smooth) {
      window.scrollTo({ top: scrollFor(stop, top, span), behavior: smooth ? 'smooth' : 'auto' });
    },
  };
}
