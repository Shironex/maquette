/**
 * The canvas's drawing resolution. The Canvas dpr range is the ceiling, a
 * pixel budget caps big windows (fill cost grows with pixels, not the
 * ratio), and a GPU that runs slow sheds more while the camera moves.
 */

/** The Canvas dpr range: R3F's rule, the device ratio clamped to it, is the ceiling. */
export const DPR_RANGE: [number, number] = [1, 1.5];
/**
 * R3F's performance factor: how far it drops while a slow GPU moves the
 * camera, and how long after the last scroll it lifts (ms). Long enough to
 * hold between the scroll events of a GPU drawing a few frames a second.
 */
export const PERFORMANCE = { min: 0.6, debounce: 500 };
/** Drawing pixels a frame may fill: about a 2560 x 1600 window at 1x. */
export const PIXEL_BUDGET = 4_000_000;
/** The ratio never goes under this, however big the window. */
const DPR_FLOOR = 0.5;
/** A frame gap longer than this, in ms, is a pause in the demand loop or a very slow frame. */
const LONG_GAP = 100;
/** A median frame time over this, in ms, is a GPU that cannot keep up (about 45 fps). */
const SLOW_FRAME = 22;
/** Frames the median is taken over, and how many it needs first. */
const WINDOW = 30;
const MIN_SAMPLES = 10;

/**
 * The ratio to draw at: the ceiling R3F would pick, capped so width x
 * height x dpr^2 stays under the budget, then scaled by R3F's performance
 * factor. Sizes are CSS pixels.
 */
export function adaptiveDpr(
  device: number,
  size: { width: number; height: number },
  factor = 1,
  range: readonly [number, number] = DPR_RANGE,
  budget = PIXEL_BUDGET
): number {
  const ceiling = Math.min(Math.max(range[0], device), range[1]);
  const area = size.width * size.height;
  const capped = area > 0 ? Math.min(ceiling, Math.sqrt(budget / area)) : ceiling;
  return Math.round(Math.max(DPR_FLOOR, capped * factor) * 100) / 100;
}

/**
 * Frame times for a demand frame loop, as a rolling median. The frame after
 * a pause times the pause, not the GPU, so a long gap is only counted when
 * the gap before it was long too (a slow GPU, not an idle loop).
 */
export class FrameTimes {
  private readonly gaps: number[] = [];
  private last: number | null = null;
  private lastLong = false;

  /** Call once per frame with its timestamp; `count` false keeps the clock but not the gap. */
  tick(now: number, count = true): void {
    const gap = this.last === null ? null : now - this.last;
    this.last = now;
    if (gap === null) return;
    const long = gap > LONG_GAP;
    if (count && (!long || this.lastLong)) {
      this.gaps.push(gap);
      if (this.gaps.length > WINDOW) this.gaps.shift();
    }
    this.lastLong = long;
  }

  /** The median gap in ms, or null until enough frames are in. */
  median(): number | null {
    if (this.gaps.length < MIN_SAMPLES) return null;
    const sorted = [...this.gaps].sort((a, b) => a - b);
    const mid = sorted.length >> 1;
    const at = (i: number) => sorted[i] ?? 0;
    return sorted.length % 2 ? at(mid) : (at(mid - 1) + at(mid)) / 2;
  }

  /** True once the median says the GPU cannot keep up. */
  slow(): boolean {
    const median = this.median();
    return median !== null && median > SLOW_FRAME;
  }
}
