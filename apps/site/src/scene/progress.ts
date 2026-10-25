/**
 * The tour's scroll progress, counted in stops: 0 on the hero frame, 1 on
 * Concept, up to 6 on Ship, fractional in between. Pure maths shared by the
 * page script and the island; no DOM and no three.js.
 */
import { STOP_IDS } from './stops';

/** Progress at the last stop (Ship). */
export const LAST_STOP = STOP_IDS.length - 1;

/** A stop's copy shows in full while progress is within this many stops of it. */
export const HOLD = 0.16;

/** A stop's copy is gone by this many stops away from it. */
export const FADE = 0.42;

export function clampProgress(progress: number, last = LAST_STOP): number {
  if (!Number.isFinite(progress)) return 0;
  return Math.min(last, Math.max(0, progress));
}

/** Progress at scroll position `y` for a tour that starts at `top` and scrolls `span` pixels. */
export function progressAt(y: number, top: number, span: number, last = LAST_STOP): number {
  if (span <= 0) return 0;
  return clampProgress(((y - top) / span) * last, last);
}

/** The scroll position that puts the tour on `stop`. */
export function scrollFor(stop: number, top: number, span: number, last = LAST_STOP): number {
  return top + (Math.max(0, span) * clampProgress(stop, last)) / last;
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
 * fading out by FADE stops away. With `cut` (reduced motion) it is all or
 * nothing on the nearest stop.
 */
export function presence(progress: number, stop: number, cut = false): number {
  if (cut) return nearestStop(progress) === stop ? 1 : 0;
  return 1 - smoothstep(Math.abs(clampProgress(progress) - stop), HOLD, FADE);
}
