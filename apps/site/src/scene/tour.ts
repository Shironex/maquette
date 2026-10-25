/**
 * The contract between the page and the 3D island. The page owns the scroll
 * position and decides the current stop; the island only listens. Both run
 * in the browser, the island much later (or never, on phones).
 */
import { STOP_IDS, type StopId } from './stops';

/** Fired on window whenever the current stop changes. */
export const STOP_EVENT = 'maquette:stop';
/** Fired by the island once the first view is loaded and drawn. */
export const READY_EVENT = 'maquette:scene-ready';
/** Fired by the island when it cannot draw (no WebGL, a lost context, a failed load). */
export const FAILED_EVENT = 'maquette:scene-failed';

export interface StopDetail {
  index: number;
  id: StopId;
}

export function stopAt(index: number): StopId {
  return STOP_IDS[Math.max(0, Math.min(STOP_IDS.length - 1, index))] ?? 'hero';
}

/** The stop the tour element currently shows, from its data attribute. */
export function currentStop(tour: HTMLElement | null): StopId {
  return stopAt(Number(tour?.dataset.stop ?? 0));
}

export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Desktop class: a fine pointer and a wide window. Phones never get the 3D. */
export const DESKTOP_QUERY = '(min-width: 1024px) and (pointer: fine)';

export function supportsWebGL2(): boolean {
  try {
    const canvas = document.createElement('canvas');
    return Boolean(canvas.getContext('webgl2'));
  } catch {
    return false;
  }
}
