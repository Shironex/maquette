/**
 * Tracks where the page is in the tour, for the page's rail and posters and
 * for the island's camera. The stop sections scroll over the pinned stage;
 * their edges are measured once and again on resize, late fonts and load, so
 * a scroll is arithmetic on scrollY with no layout reads. No three.js imports.
 */
import { progressFromEdges, scrollForProgress } from './progress';
import type { TourScroll } from './tour';

export interface TourTracker extends TourScroll {
  /** Scrolls the page to a stop's section. */
  scrollToStop(stop: number, smooth: boolean): void;
  /** Scrolls the page to fractional progress, for Maquette's scrubber. */
  scrollToProgress(progress: number, smooth: boolean): void;
}

/**
 * Starts tracking `tour` and its stop `sections`, in order. While `active`
 * does not match (narrow windows, where the stops stack) progress stays 0.
 */
export function trackTour(
  tour: HTMLElement,
  sections: readonly HTMLElement[],
  active: MediaQueryList
): TourTracker {
  const listeners = new Set<(progress: number) => void>();
  let edges: number[] = [];
  let progress = 0;

  const update = () => {
    const next = active.matches ? progressFromEdges(window.scrollY, edges) : 0;
    if (next === progress) return;
    progress = next;
    for (const listener of listeners) listener(progress);
  };
  const measure = () => {
    const y = window.scrollY;
    edges = sections.map(section => section.getBoundingClientRect().top + y);
    const last = sections.at(-1);
    if (last) edges.push((edges.at(-1) ?? 0) + last.offsetHeight);
    update();
  };

  measure();
  window.addEventListener('scroll', update, { passive: true });
  window.addEventListener('resize', measure, { passive: true });
  active.addEventListener('change', measure);
  new ResizeObserver(measure).observe(tour);
  // Late fonts and images move the sections without always resizing the tour.
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
      const top = edges[Math.max(0, Math.min(sections.length - 1, Math.round(stop)))] ?? 0;
      window.scrollTo({ top, behavior: smooth ? 'smooth' : 'auto' });
    },
    scrollToProgress(at, smooth) {
      const top = scrollForProgress(at, edges);
      window.scrollTo({ top, behavior: smooth ? 'smooth' : 'auto' });
    },
  };
}
