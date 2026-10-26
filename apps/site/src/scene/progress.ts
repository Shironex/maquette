/**
 * The tour's scroll progress, counted in stops: 0 on the hero frame, 1 on
 * Concept, up to 6 on Ship, fractional in between, measured from the
 * stop sections that scroll over the pinned stage. Pure maths shared by the
 * page script and the island; no DOM and no three.js.
 */
import { STOP_IDS } from './stops';

/** Progress at the last stop (Ship). */
export const LAST_STOP = STOP_IDS.length - 1;

/** A stop's effects (the hero callouts, the wireframe sweep) show in full within this many stops of it. */
export const HOLD = 0.16;

/** A stop's effects are gone by this many stops away from it. */
export const FADE = 0.42;

export function clampProgress(progress: number, last = LAST_STOP): number {
  if (!Number.isFinite(progress)) return 0;
  return Math.min(last, Math.max(0, progress));
}

/**
 * Progress at scroll position `y` from the page positions of the stop
 * sections: `edges` holds each section's top, then the last one's bottom.
 * A stop's progress is reached as its section's top meets the top of the
 * window, so its copy (centred in the section) is centred on screen.
 */
export function progressFromEdges(y: number, edges: readonly number[]): number {
  const last = edges.length - 2;
  if (last < 1) return 0;
  for (let i = last; i >= 0; i--) {
    const top = edges[i] ?? 0;
    if (top > y) continue;
    const height = (edges[i + 1] ?? top) - top;
    return clampProgress(height > 0 ? i + (y - top) / height : i, last);
  }
  return 0;
}

/**
 * The scroll position that reaches `progress`, the inverse of
 * progressFromEdges: the top of its stop's section plus the share of the
 * section's height the fraction asks for.
 */
export function scrollForProgress(progress: number, edges: readonly number[]): number {
  const last = edges.length - 2;
  if (last < 0) return 0;
  const at = clampProgress(progress, Math.max(0, last));
  const index = Math.min(last, Math.floor(at));
  const top = edges[index] ?? 0;
  const height = (edges[index + 1] ?? top) - top;
  return top + (at - index) * height;
}

/** The stop the page names as current: the nearest one. */
export function nearestStop(progress: number, last = LAST_STOP): number {
  return Math.round(clampProgress(progress, last));
}

/** 0 at or below `min`, 1 at or above `max`, eased in and out between. */
export function smoothstep(x: number, min: number, max: number): number {
  if (x <= min) return 0;
  if (x >= max) return 1;
  const k = (x - min) / (max - min);
  return k * k * (3 - 2 * k);
}

/** 0 before `from`, 1 after `to`, straight in between. */
export function across(progress: number, from: number, to: number): number {
  if (to <= from) return progress < from ? 0 : 1;
  return Math.min(1, Math.max(0, (progress - from) / (to - from)));
}

/**
 * How present stop `stop` is, 0 to 1: full while the tour holds on it,
 * fading out by FADE stops away. With `cut` it is all or nothing on the
 * nearest stop.
 */
export function presence(progress: number, stop: number, cut = false): number {
  if (cut) return nearestStop(progress) === stop ? 1 : 0;
  return 1 - smoothstep(Math.abs(clampProgress(progress) - stop), HOLD, FADE);
}
