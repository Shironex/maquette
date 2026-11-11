/**
 * The geometry of the schematic hand GripPicture draws: points for the
 * thumb and each finger on a 60 by 80 box, the palm facing the viewer and
 * the fingers up. A finger joint's curl (its turn about Z, as the grip
 * library keeps them) shortens the segment past it, as a finger closing
 * toward the viewer looks; the hand bone's Z tilts the whole hand.
 */
import type { GripSide } from '@/ipc/client';

type Point = [number, number];

export interface HandPicture {
  /** The thumb first, then index to little, each from its base outward. */
  fingers: Point[][];
  /** Degrees the hand tilts about the wrist. */
  tilt: number;
}

const FINGERS: readonly (readonly [name: string, x: number, length: number])[] = [
  ['Index', 18, 22],
  ['Middle', 26.5, 25],
  ['Ring', 35, 23],
  ['Little', 43, 17],
];
const JOINTS = ['Proximal', 'Intermediate', 'Distal'] as const;
const SHARES = [0.45, 0.32, 0.23] as const;
const THUMB = ['Metacarpal', 'Proximal', 'Distal'] as const;
const THUMB_LENGTHS = [9, 8, 7] as const;

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);
const round = (v: number) => Math.round(v * 10) / 10;

/** The turn about `axis` of `bone`, 0 when the pose leaves it out. */
function turn(pose: Record<string, readonly (number | null)[]>, bone: string, axis: number) {
  const v = pose[bone]?.[axis];
  return typeof v === 'number' && Number.isFinite(v) ? v : 0;
}

/**
 * The hand on `side` of `pose`. The right hand is drawn with its thumb on
 * the left of the box; the left hand is its mirror, its curls read with
 * the sign the mirror gives them.
 */
export function handPicture(
  pose: Record<string, readonly (number | null)[]>,
  side: GripSide
): HandPicture {
  const flip = side === 'left' ? -1 : 1;
  const across = (x: number) => (side === 'left' ? 60 - x : x);
  const fingers = FINGERS.map(([name, x, length]) => {
    let curl = 0;
    let y = 36;
    const points: Point[] = [[round(across(x)), y]];
    JOINTS.forEach((joint, j) => {
      curl = clamp(curl + Math.abs(turn(pose, `${side}${name}${joint}`, 2)), 0, 170);
      y -= length * SHARES[j]! * Math.cos((curl * Math.PI) / 180);
      points.push([round(across(x)), round(y)]);
    });
    return points;
  });
  let angle = -125;
  let at: Point = [12, 56];
  const thumb: Point[] = [[round(across(at[0])), at[1]]];
  THUMB.forEach((joint, j) => {
    const fold = Math.abs(turn(pose, `${side}Thumb${joint}`, 1));
    angle += clamp(fold, 0, 90) * 0.6;
    const length = THUMB_LENGTHS[j]!;
    at = [
      at[0] + length * Math.cos((angle * Math.PI) / 180),
      at[1] + length * Math.sin((angle * Math.PI) / 180),
    ];
    thumb.push([round(across(at[0])), round(at[1])]);
  });
  return {
    fingers: [thumb, ...fingers],
    tilt: round(clamp(turn(pose, `${side}Hand`, 2), -40, 40) * flip),
  };
}
