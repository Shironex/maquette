/**
 * The real size maths the core does, for the UI and mock mode: which side a
 * size measures, the uniform scale that makes a model that size and the
 * frame the origin fix-up and the scale give a model's points.
 */
import type { ModelPrep, RealSize, SizeAxis, SizeUnit } from '@/ipc/client';

export type Extent = [number, number, number];
type Maybe = number | null | undefined;

const UNIT_METRES: Record<SizeUnit, number> = { cm: 0.01, m: 1 };

/** The smallest and largest size the core accepts, in metres. */
export const MIN_METRES = 0.001;
export const MAX_METRES = 1000;

export const metres = (size: RealSize): number => (size.value ?? 0) * UNIT_METRES[size.unit];

/** Width (X), height (Y) and depth (Z) of bounds. */
export function extentOf(min: readonly Maybe[], max: readonly Maybe[]): Extent {
  return [0, 1, 2].map(i => (max[i] ?? 0) - (min[i] ?? 0)) as Extent;
}

/** The side `axis` measures, in metres. */
export function measuredAlong(axis: SizeAxis, extent: Extent): number {
  if (axis === 'width') return extent[0];
  if (axis === 'height') return extent[1];
  if (axis === 'depth') return extent[2];
  return Math.max(...extent);
}

/** The uniform scale that makes a model of `extent` measure `size`, or null when that side is flat. */
export function scaleFor(size: RealSize, extent: Extent): number | null {
  const measured = measuredAlong(size.axis, extent);
  const target = metres(size);
  const scale = target / measured;
  return measured > 1e-6 && target > 0 && Number.isFinite(scale) ? scale : null;
}

/** What preparing does to a model's points: `p' = scale * (p + offset)`. */
export interface Frame {
  scale: number;
  offset: Extent;
}

export const IDENTITY: Frame = { scale: 1, offset: [0, 0, 0] };

/** The frame `prep` gives a model with these bounds, as the core works it out. */
export function frameOf(
  prep: ModelPrep | null | undefined,
  min: readonly Maybe[] | null,
  max: readonly Maybe[] | null
): Frame {
  if (!prep || !min || !max) return IDENTITY;
  const lo = [0, 1, 2].map(i => min[i] ?? 0);
  const hi = [0, 1, 2].map(i => max[i] ?? 0);
  const origin = prep.fixups.some(f => f.op === 'origin-bottom');
  const offset: Extent = origin
    ? [-(lo[0]! + hi[0]!) / 2, -lo[1]!, -(lo[2]! + hi[2]!) / 2]
    : [0, 0, 0];
  const scale = prep.size ? (scaleFor(prep.size, extentOf(lo, hi)) ?? 1) : 1;
  return { scale, offset };
}

/** A point through `frame`. */
export function framed(frame: Frame, p: readonly Maybe[]): Extent {
  return [0, 1, 2].map(i => frame.scale * ((p[i] ?? 0) + frame.offset[i]!)) as Extent;
}

/** "45 cm", "1.2 m", "8 mm": centimetres under a metre, millimetres under one. */
export function formatLength(m: number): string {
  const trim = (v: number, digits: number) => String(Number(v.toFixed(digits)));
  if (!Number.isFinite(m) || m <= 0) return '0 cm';
  if (m >= 1) return `${trim(m, 2)} m`;
  if (m < 0.01) return `${trim(m * 1000, 1)} mm`;
  return `${trim(m * 100, 1)} cm`;
}

/** "62 × 158 × 31 cm" or "11.7 × 8 × 7.6 m", in the unit of the largest side. */
export function formatDims(extent: Extent): string {
  const big = Math.max(...extent) >= 1;
  const show = (v: number) => String(Number((big ? v : v * 100).toFixed(big ? 2 : 1)));
  return `${extent.map(show).join(' × ')} ${big ? 'm' : 'cm'}`;
}
