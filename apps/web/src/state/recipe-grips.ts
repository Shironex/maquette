/**
 * Grips: a hand pose kept in the grip library and laid on a recipe as a copy
 * with a link back. A step for step copy of the core's rules
 * (`recipes/grips.rs` and its `apply`, `save` and `span` parts, cases both
 * agree on in `fixtures/grips.json`): the bones a grip may pose, its mirror,
 * which hands a recipe takes from one, the change that holds the copy, a
 * newer revision taken in, the grip a pose key change makes at a frame, and
 * a grip change on the timeline's frames. Pure helpers.
 */
import type {
  GripFile,
  GripFrom,
  GripHands,
  GripHolds,
  GripSide,
  LayerMode,
  RecipeFile,
  RecipeLayer,
} from '@/ipc/client';
import { mirrorPose } from './recipe-key-edits';
import { layerTurns, shareAt } from './recipe-keys';
import { eased, framesWithin } from './recipe-pins';
import type { Timeline } from './recipe-timeline';

/** Degrees per VRM humanoid bone, X then Y then Z, as a grip holds them. */
export type GripPose = GripFile['pose'];

/** The newest grip file version. */
export const GRIP_VERSION = 1;
/** The frames a grip eases in and out over unless told. */
export const GRIP_EASE_FRAMES = 4;

/** What a grip holds: its id in the file, how it reads and the group it sorts under. */
export const WEAPONS: readonly (readonly [id: string, label: string, group: string])[] = [
  ['longsword', 'Longsword', 'swords'],
  ['sword', 'Sword', 'swords'],
  ['rapier', 'Rapier', 'swords'],
  ['scythe', 'Scythe', 'polearms'],
  ['staff', 'Staff', 'polearms'],
  ['spear', 'Spear', 'polearms'],
  ['dagger', 'Dagger', 'daggers'],
  ['shield', 'Shield', 'shields'],
  ['bow', 'Bow', 'other'],
  ['empty', 'Empty hand', 'empty'],
  ['other', 'Something else', 'other'],
];

/** The groups the library sorts grips under, in order. */
export const WEAPON_GROUPS: readonly (readonly [id: string, label: string])[] = [
  ['swords', 'Swords'],
  ['polearms', 'Polearms'],
  ['daggers', 'Daggers'],
  ['shields', 'Shields'],
  ['empty', 'Empty'],
  ['other', 'Other'],
];

/** The group `weapon` sorts under; `other` for one this Maquette does not know. */
export const weaponGroup = (weapon: string): string =>
  WEAPONS.find(([id]) => id === weapon)?.[2] ?? 'other';

/** "Longsword", or the id itself for a weapon this Maquette does not know. */
export const weaponLabel = (weapon: string): string =>
  WEAPONS.find(([id]) => id === weapon)?.[1] ?? weapon;

const ARM = ['UpperArm', 'LowerArm', 'Hand'] as const;
const FINGERS: readonly (readonly [string, readonly string[]])[] = [
  ['Thumb', ['Metacarpal', 'Proximal', 'Distal']],
  ['Index', ['Proximal', 'Intermediate', 'Distal']],
  ['Middle', ['Proximal', 'Intermediate', 'Distal']],
  ['Ring', ['Proximal', 'Intermediate', 'Distal']],
  ['Little', ['Proximal', 'Intermediate', 'Distal']],
];

/** The side `bone` is on, by its VRM name; null for a bone in the middle. */
export const boneSide = (bone: string): GripSide | null =>
  bone.startsWith('left') ? 'left' : bone.startsWith('right') ? 'right' : null;

const partOf = (bone: string): string | null => {
  const side = boneSide(bone);
  return side ? bone.slice(side.length) : null;
};

/** Whether a grip may pose `bone`: a hand, a finger bone, or the arm above the hand. */
export function isGripBone(bone: string): boolean {
  const part = partOf(bone);
  if (part === null) return false;
  if ((ARM as readonly string[]).includes(part)) return true;
  return FINGERS.some(
    ([finger, segments]) => part.startsWith(finger) && segments.includes(part.slice(finger.length))
  );
}

/** Every bone a grip may pose on `side`, the arm first, then each finger from its base. */
export const gripBones = (side: GripSide): string[] => [
  ...ARM.map(part => `${side}${part}`),
  ...FINGERS.flatMap(([finger, segments]) => segments.map(s => `${side}${finger}${s}`)),
];

/** The other side. */
export const otherHand = (side: GripSide): GripSide => (side === 'left' ? 'right' : 'left');

/** The hand on `side`, by VRM name. */
export const handBone = (side: GripSide) => `${side}Hand`;

/** `pose` on the other side of the body, as a mirror shows it. */
export const mirrorGrip = (pose: GripPose): GripPose => mirrorPose(pose);

/** Whether `pose` turns a bone on the left, and on the right. */
export function sidesOf(pose: GripPose): { left: boolean; right: boolean } {
  const bones = Object.keys(pose);
  return {
    left: bones.some(b => boneSide(b) === 'left'),
    right: bones.some(b => boneSide(b) === 'right'),
  };
}

/** The bones of `pose` on `side`. */
export const sidePose = (pose: GripPose, side: GripSide): GripPose =>
  Object.fromEntries(Object.entries(pose).filter(([bone]) => boneSide(bone) === side));

/** How a grip's hands read: both hands, right hand, left hand, or either hand for an empty one. */
export function handsWords(grip: Pick<GripFile, 'hands' | 'weapon'>): string {
  if (grip.hands.supports) return 'both hands';
  if (grip.weapon === 'empty') return 'either hand';
  return grip.hands.holds === 'right' ? 'right hand' : 'left hand';
}

/** The hands a recipe takes from `grip` unless told: both, else the one it poses. */
export const defaultHands = (grip: Pick<GripFile, 'hands'>): GripHands =>
  grip.hands.supports ? 'both' : grip.hands.holds;

/** The hands `pose` turns: both, one, or null for a pose with no bone on either side. */
export function handsOfPose(pose: GripPose): GripHands | null {
  const { left, right } = sidesOf(pose);
  if (left && right) return 'both';
  return left ? 'left' : right ? 'right' : null;
}

/** The bones of `pose` on `side`, or the other side's in a mirror when it turns none there. */
function onSide(pose: GripPose, side: GripSide): GripPose {
  const own = sidePose(pose, side);
  return Object.keys(own).length ? own : mirrorGrip(sidePose(pose, otherHand(side)));
}

/** The pose a recipe takes from `grip` for `hands` (its own unless told), and the hand that holds. */
export function poseFor(
  grip: Pick<GripFile, 'hands' | 'pose'>,
  hands?: GripHands | null
): { holds: GripSide; pose: GripPose } {
  const taken = hands ?? defaultHands(grip);
  if (taken === 'both')
    return {
      holds: grip.hands.holds,
      pose: { ...onSide(grip.pose, 'left'), ...onSide(grip.pose, 'right') },
    };
  return { holds: taken, pose: onSide(grip.pose, taken) };
}

/**
 * A new grip change `id` holding a copy of `grip` for `hands`, linked back
 * with `from`: on the hand that holds, in place of the clip's own turns,
 * across the whole recipe, easing in and out over the default frames.
 */
export function layerOf(
  id: string,
  grip: Pick<GripFile, 'hands' | 'pose'>,
  from: GripFrom,
  hands?: GripHands | null
): RecipeLayer {
  const { holds, pose } = poseFor(grip, hands);
  return {
    id,
    kind: 'grip',
    bone: handBone(holds),
    mode: 'replace',
    pose,
    from,
    settings: {
      weight: 1,
      start: 0,
      end: 1,
      target: null,
      rotation: null,
      offset: null,
      cycles: null,
      amount: null,
      easeIn: GRIP_EASE_FRAMES,
      easeOut: GRIP_EASE_FRAMES,
    },
  };
}

/**
 * `layer`, a grip change, taking `grip` with the link `from`: where it sits,
 * its eases, weight and what it holds stay; its pose is the grip's for
 * `hands`, or for the hands it posed before.
 */
export function updated(
  layer: RecipeLayer,
  grip: Pick<GripFile, 'hands' | 'pose'>,
  from: GripFrom,
  hands?: GripHands | null
): RecipeLayer {
  const { holds, pose } = poseFor(grip, hands ?? (layer.pose ? handsOfPose(layer.pose) : null));
  return { ...layer, bone: handBone(holds), pose, from };
}

const stored = (v: number) => {
  const r = Math.round(v * 100) / 100;
  return r === 0 ? 0 : r;
};

/**
 * The hand pose the pose key change `layer` of `recipe` holds at bake frame
 * `frame`: each hand, finger and arm bone it keys, as blended between its
 * keys there, to a hundredth of a degree. With `keep`, only those bones.
 * Null when it turns none.
 */
export function keyedPose(
  recipe: RecipeFile,
  t: Timeline,
  layer: RecipeLayer,
  frame: number,
  keep?: readonly string[] | null
): GripPose | null {
  if (layer.kind !== 'pose') return null;
  const share = shareAt(recipe, t, Math.min(frame, t.frames));
  const pose: GripPose = {};
  for (const [bone, turn] of layerTurns(recipe, layer, share)) {
    if (!isGripBone(bone) || (keep && !keep.includes(bone))) continue;
    pose[bone] = [stored(turn[0]), stored(turn[1]), stored(turn[2])];
  }
  return Object.keys(pose).length ? pose : null;
}

/**
 * The hands and pose a grip made from `pose` has for `hands`: one hand
 * keeps only its side's bones; both needs bones on both sides, the right
 * hand holding unless `holds` says the left. Left out, the hands are the
 * sides the pose turns. Null when the pose turns no bone on a side asked for.
 */
export function gripOf(
  pose: GripPose,
  hands?: GripHands | null,
  holds?: GripSide | null
): { hands: GripHolds; pose: GripPose } | null {
  const { left, right } = sidesOf(pose);
  const taken = hands ?? (left && right ? 'both' : left ? 'left' : 'right');
  if (taken === 'both') {
    if (!left || !right) return null;
    const holding = holds ?? 'right';
    return { hands: { holds: holding, supports: otherHand(holding) }, pose };
  }
  if (taken === 'left' ? !left : !right) return null;
  return { hands: { holds: taken, supports: null }, pose: sidePose(pose, taken) };
}

/** One grip change on the timeline's frames, as the bake and the preview lay it. */
export interface GripSpan {
  layer: string;
  bone: string;
  first: number;
  last: number;
  easeIn: number;
  easeOut: number;
  weight: number;
  mode: LayerMode;
}

const n = (v: number | null | undefined, or: number) =>
  typeof v === 'number' && Number.isFinite(v) ? v : or;

/** Where `layer`, a grip, holds on timeline `t`; null when no frame falls within its range. */
export function spanOn(t: Timeline, layer: RecipeLayer): GripSpan | null {
  const s = layer.settings;
  const frames = framesWithin(t, n(s.start, 0), n(s.end, 1));
  if (!frames) return null;
  return {
    layer: layer.id,
    bone: layer.bone,
    first: frames[0],
    last: frames[1],
    easeIn: s.easeIn ?? GRIP_EASE_FRAMES,
    easeOut: s.easeOut ?? GRIP_EASE_FRAMES,
    weight: Math.min(Math.max(n(s.weight, 1), 0), 1),
    mode: layer.mode ?? 'replace',
  };
}

/** Every grip of `layers` on timeline `t`, in the order they are laid. */
export const gripSpans = (t: Timeline, layers: readonly RecipeLayer[]): GripSpan[] =>
  layers.flatMap(l => (l.kind === 'grip' ? (spanOn(t, l) ?? []) : []));

/** How much of `grip` shows at timeline frame `frame`, 0 to 1, weight in. */
export const gripAmount = (grip: GripSpan, frame: number): number =>
  grip.weight * eased(grip, frame);
