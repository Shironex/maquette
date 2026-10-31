/**
 * The timeline's page wide tracks: the look the 3D layer fades into and how
 * much of the hero copy shows. Pure maths on tour progress; PageTracks.tsx
 * writes the results. The look is a CSS filter on the canvas, so it adds no
 * draw calls and needs no second set of materials.
 */
import { keyedAt, type Easing } from './timeline';

export type TrackLook = 'blueprint' | 'toon' | 'clay';

export interface StyleTrack {
  look: TrackLook;
  easing: Easing;
  keys: { stop: string; amount: number | null }[];
}

export interface TextTrack {
  easing: Easing;
  keys: { stop: string; amount: number | null }[];
}

const fixed = (v: number) => Number(v.toFixed(3));

/** The canvas's filter at `amount` of the look: 0 as built, 1 the whole look. */
export function lookFilter(look: TrackLook, amount: number): string {
  const a = Math.min(1, Math.max(0, amount));
  if (a <= 0) return 'none';
  switch (look) {
    case 'blueprint':
      return `grayscale(${fixed(a)}) sepia(${fixed(a)}) hue-rotate(${fixed(170 * a)}deg) saturate(${fixed(1 + 1.5 * a)}) brightness(${fixed(1 - 0.1 * a)})`;
    case 'toon':
      return `saturate(${fixed(1 + 0.6 * a)}) contrast(${fixed(1 + 0.35 * a)})`;
    case 'clay':
      return `grayscale(${fixed(a)}) sepia(${fixed(0.25 * a)}) brightness(${fixed(1 + 0.05 * a)})`;
  }
}

/** The filter at tour progress `progress`; 'none' with no track. */
export function styleAt(
  track: StyleTrack | null | undefined,
  stopIds: readonly string[],
  progress: number
): string {
  if (!track?.keys.length) return 'none';
  return lookFilter(track.look, keyedAt(track, stopIds, progress));
}

/** The hero copy's opacity at tour progress `progress`; fully shown with no track. */
export function textAt(
  track: TextTrack | null | undefined,
  stopIds: readonly string[],
  progress: number
): number {
  return track?.keys.length ? keyedAt(track, stopIds, progress) : 1;
}
