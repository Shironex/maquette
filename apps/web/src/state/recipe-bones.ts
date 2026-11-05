/**
 * The humanoid bones a recipe change may move, by VRM name, as the core
 * checks them (crates/maquette-model/src/recipes/check.rs), and the
 * humanoid (Mixamo) name the retarget bone map lists each under.
 */
import type { LayerKind } from '@/ipc/client';

/** The hands and feet a reach moves. */
export const REACH_BONES = ['leftHand', 'rightHand', 'leftFoot', 'rightFoot'] as const;
/** What turns to look. */
export const LOOK_BONES = ['head', 'neck'] as const;
/** The arm bones a wave swings. */
export const WAVE_BONES = [
  'leftUpperArm',
  'rightUpperArm',
  'leftLowerArm',
  'rightLowerArm',
  'leftHand',
  'rightHand',
] as const;

/** VRM body bones with their Mixamo names; `{side}` reads Left or Right. */
const BODY: readonly (readonly [string, string])[] = [
  ['hips', 'Hips'],
  ['spine', 'Spine'],
  ['chest', 'Spine1'],
  ['upperChest', 'Spine2'],
  ['neck', 'Neck'],
  ['head', 'Head'],
  ['jaw', ''],
  ['{side}Eye', '{Side}Eye'],
  ['{side}UpperLeg', '{Side}UpLeg'],
  ['{side}LowerLeg', '{Side}Leg'],
  ['{side}Foot', '{Side}Foot'],
  ['{side}Toes', '{Side}ToeBase'],
  ['{side}Shoulder', '{Side}Shoulder'],
  ['{side}UpperArm', '{Side}Arm'],
  ['{side}LowerArm', '{Side}ForeArm'],
  ['{side}Hand', '{Side}Hand'],
];

/** VRM finger, its three segments, and Mixamo's finger name (joints 1 to 3). */
const FINGERS: readonly (readonly [string, readonly string[], string])[] = [
  ['Thumb', ['Metacarpal', 'Proximal', 'Distal'], 'Thumb'],
  ['Index', ['Proximal', 'Intermediate', 'Distal'], 'Index'],
  ['Middle', ['Proximal', 'Intermediate', 'Distal'], 'Middle'],
  ['Ring', ['Proximal', 'Intermediate', 'Distal'], 'Ring'],
  ['Little', ['Proximal', 'Intermediate', 'Distal'], 'Pinky'],
];

const SIDES = [
  ['left', 'Left'],
  ['right', 'Right'],
] as const;

function table(): Map<string, string> {
  const out = new Map<string, string>();
  for (const [vrm, mixamo] of BODY) {
    if (!vrm.includes('{side}')) {
      out.set(vrm, mixamo);
      continue;
    }
    for (const [side, Side] of SIDES)
      out.set(vrm.replace('{side}', side), mixamo.replace('{Side}', Side));
  }
  for (const [side, Side] of SIDES)
    for (const [finger, segments, mixamo] of FINGERS)
      segments.forEach((segment, i) =>
        out.set(`${side}${finger}${segment}`, `${Side}Hand${mixamo}${i + 1}`)
      );
  return out;
}

const MIXAMO = table();

/** Every VRM humanoid bone, the body first, then each side's fingers. */
export const HUMAN_BONES: readonly string[] = [...MIXAMO.keys()];

/** The bones a change of `kind` may move. */
export function bonesFor(kind: LayerKind): readonly string[] {
  switch (kind) {
    case 'reach':
      return REACH_BONES;
    case 'look-at':
      return LOOK_BONES;
    case 'wave':
      return WAVE_BONES;
    case 'nudge':
      return HUMAN_BONES;
  }
}

/** The humanoid (Mixamo) name the bone map lists `vrm` under, or null. */
export const mixamoName = (vrm: string): string | null => MIXAMO.get(vrm) || null;

/** The bone a reach bends at the middle of, and the one it turns at the top, by VRM name. */
export function limbOf(end: string): { upper: string; lower: string } | null {
  const match = /^(left|right)(Hand|Foot)$/.exec(end);
  if (!match) return null;
  const [, side, part] = match;
  return part === 'Hand'
    ? { upper: `${side}UpperArm`, lower: `${side}LowerArm` }
    : { upper: `${side}UpperLeg`, lower: `${side}LowerLeg` };
}

/** "rightHand" reads "Right hand", "leftUpperArm" reads "Left upper arm". */
export function boneWords(vrm: string): string {
  const words = vrm.replace(/([A-Z])/g, ' $1').toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
}
