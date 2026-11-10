/**
 * How far a humanoid bone may turn under its parent before a pose looks
 * broken: one fixed table, read in a pose key's turns (degrees about the
 * model's own axes). It only warns, and only keyed poses are judged. A copy
 * of the core's table (crates/maquette-model/src/recipes/joint_limits.rs),
 * checked against fixtures/pose_keys.json there. The table names the left
 * side; the right is its mirror (X as it is, Y and Z flipped).
 */
import type { Turn } from './recipe-keys';

export interface JointLimits {
  x: [number, number];
  y: [number, number];
  z: [number, number];
}

const l = (x: [number, number], y: [number, number], z: [number, number]): JointLimits => ({
  x,
  y,
  z,
});

/** The body, middle and left side; the hips move the whole body, so they have none. */
const BODY: Record<string, JointLimits> = {
  spine: l([-30, 45], [-40, 40], [-30, 30]),
  chest: l([-25, 40], [-45, 45], [-30, 30]),
  upperChest: l([-20, 30], [-30, 30], [-20, 20]),
  neck: l([-40, 50], [-60, 60], [-40, 40]),
  head: l([-40, 40], [-70, 70], [-35, 35]),
  jaw: l([-5, 30], [-10, 10], [-10, 10]),
  leftEye: l([-25, 25], [-35, 35], [-5, 5]),
  leftShoulder: l([-20, 20], [-30, 30], [-15, 30]),
  leftUpperArm: l([-90, 90], [-110, 50], [-95, 110]),
  leftLowerArm: l([-90, 90], [-150, 5], [-15, 15]),
  leftHand: l([-40, 40], [-80, 80], [-70, 70]),
  leftUpperLeg: l([-120, 30], [-45, 45], [-30, 60]),
  leftLowerLeg: l([-5, 150], [-15, 15], [-15, 15]),
  leftFoot: l([-50, 30], [-30, 30], [-30, 30]),
  leftToes: l([-40, 60], [-10, 10], [-10, 10]),
};

const FINGER = l([-15, 15], [-25, 25], [-100, 20]);
const THUMB = l([-60, 60], [-60, 60], [-60, 60]);
const FINGER_RE = /^left(Index|Middle|Ring|Little)(Proximal|Intermediate|Distal)$/;
const THUMB_RE = /^leftThumb(Metacarpal|Proximal|Distal)$/;

function leftLimits(bone: string): JointLimits | null {
  return (
    BODY[bone] ?? (THUMB_RE.test(bone) ? THUMB : null) ?? (FINGER_RE.test(bone) ? FINGER : null)
  );
}

/** How far `bone` may turn, or null for one the table leaves free. */
export function jointLimits(bone: string): JointLimits | null {
  if (!bone.startsWith('right')) return leftLimits(bone);
  const m = leftLimits(`left${bone.slice(5)}`);
  if (!m) return null;
  const flip = ([a, b]: [number, number]): [number, number] => [-b, -a];
  return { x: m.x, y: flip(m.y), z: flip(m.z) };
}

const ARM_PARTS = ['Shoulder', 'Arm', 'Hand', 'Thumb', 'Index', 'Middle', 'Ring', 'Little'];

/** What each axis does to `bone`: an upright bone bends, twists, leans; an arm twists, swings, raises. */
export const axisWords = (bone: string): [string, string, string] =>
  ARM_PARTS.some(p => bone.includes(p)) ? ['twist', 'swing', 'raise'] : ['bend', 'twist', 'side'];

export interface LimitHit {
  axis: 'x' | 'y' | 'z';
  word: string;
  degrees: number;
  limit: number;
}

/** Each axis of `turn` past `bone`'s limits. */
export function overLimits(bone: string, turn: Turn): LimitHit[] {
  const limits = jointLimits(bone);
  if (!limits) return [];
  const words = axisWords(bone);
  return (['x', 'y', 'z'] as const).flatMap((axis, i) => {
    const [low, high] = limits[axis];
    const degrees = turn[i]!;
    if (degrees >= low && degrees <= high) return [];
    return [{ axis, word: words[i]!, degrees, limit: degrees < low ? low : high }];
  });
}

const deg = (v: number) => String(Math.round(Math.abs(v)));

/**
 * "Chest twist 18 of 45 degrees, bend 12 of 40.": each axis the turn uses,
 * against the limit on its side; null for a bone with no limits or no turn.
 */
export function limitLine(bone: string, turn: Turn, name: string): string | null {
  const limits = jointLimits(bone);
  if (!limits) return null;
  const words = axisWords(bone);
  const parts = (['x', 'y', 'z'] as const).flatMap((axis, i) => {
    const v = turn[i]!;
    if (Math.abs(v) < 0.5) return [];
    const [low, high] = limits[axis];
    return [`${words[i]} ${deg(v)} of ${deg(v < 0 ? low : high)}`];
  });
  if (!parts.length) return null;
  return `${name} ${parts.join(', ')}`.replace(/ of (\d+)/, ' of $1 degrees') + '.';
}
