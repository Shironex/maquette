/**
 * The contract between the page and the 3D island. The page owns the scroll
 * position and turns it into tour progress; the island only reads it. Both
 * run in the browser, the island much later (or never, on phones).
 */
import { STOP_IDS, type StopId } from './stops';

/** Tour progress in stops (see ./progress.ts), as the page measures it. */
export interface TourScroll {
  /** The current progress. Cheap enough to read every frame. */
  read(): number;
  /** Calls `listener` now and on every change; returns the unsubscribe. */
  subscribe(listener: (progress: number) => void): () => void;
}

/** Fired by the island once the first view is loaded and drawn. */
export const READY_EVENT = 'maquette:scene-ready';
/** Fired by the island when it cannot draw (no WebGL, a lost context, a failed load). */
export const FAILED_EVENT = 'maquette:scene-failed';

export function stopAt(index: number): StopId {
  return STOP_IDS[Math.max(0, Math.min(STOP_IDS.length - 1, index))] ?? 'hero';
}

export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Desktop class: a fine pointer and a wide window. Phones never get the 3D. */
export const DESKTOP_QUERY = '(min-width: 1024px) and (pointer: fine)';

/**
 * Context attributes the probe and the scene's renderer share. A software
 * renderer still makes a WebGL 2 context but draws the scene at a few frames
 * a second; with the caveat flag it refuses, and the page keeps its posters.
 */
export const GPU_CONTEXT: WebGLContextAttributes = { failIfMajorPerformanceCaveat: true };

export function supportsWebGL2(): boolean {
  try {
    const gl = document.createElement('canvas').getContext('webgl2', GPU_CONTEXT);
    // Give the context back at once; the scene makes its own.
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
    return Boolean(gl);
  } catch {
    return false;
  }
}
