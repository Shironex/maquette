/**
 * The combat starters in mock mode, as the core builds them in
 * (starters/combat.rs): a two cut sword combo and a charged two handed
 * scythe swing, each with a left handed copy that only mirrors. Clip names
 * only, tried in order; pose keys, the hold and the blends are laid as
 * shares of the clips found, so they fit a clip of any length.
 */
import type { AnimationClip, PoseKey, RecipeBase, RecipeFile, RecipeLayer } from '../client';
import { mirrorGrip, type GripPose } from '@/state/recipe-grips';
import { REST_FPS, baseOf, clipNamed, layer, namedBase, newRecipe } from './recipe-files';

interface Join {
  names: string[];
  /** How long to take it to be when no clip has one of its names. */
  frames: number;
  blend: number;
}

export interface Combo {
  key: string;
  name: string;
  about: string;
  clips: Join[];
  mirror: boolean;
  shape: (r: RecipeFile) => void;
}

const IDLE = ['Idle', 'Breathing Idle'];
const SLASH_CLIPS: Join[] = [
  { names: ['Sword And Shield Slash', 'Sword Slash', 'Slash'], frames: 40, blend: 0 },
  {
    names: ['Sword And Shield Attack', 'Sword And Shield Slash', 'Sword Slash', 'Slash'],
    frames: 40,
    blend: 6,
  },
  { names: ['Sword And Shield Idle', ...IDLE], frames: 60, blend: 10 },
];
const SCYTHE_CLIPS: Join[] = [
  {
    names: [
      'Great Sword Slash',
      'Two Handed Slash',
      'Standing Melee Attack Horizontal',
      'Sword And Shield Slash',
    ],
    frames: 50,
    blend: 0,
  },
  { names: ['Great Sword Idle', ...IDLE], frames: 60, blend: 10 },
];

/** The frame `share` of the way through clip `clip` of `r`. */
function at(r: RecipeFile, clip: number, share: number): number {
  const c = clip === 0 ? r.base : r.joined?.[clip - 1];
  return Math.round(Math.max((c?.frames ?? 1) - 1, 0) * share);
}

type Turn = [number, number, number];

const key = (
  r: RecipeFile,
  clip: number,
  share: number,
  ease: PoseKey['ease'],
  pose: Record<string, Turn>
): PoseKey => ({ clip, frame: at(r, clip, share), ease, pose });

const keyed = (keys: PoseKey[]): RecipeLayer => ({
  ...layer('l-1', 'pose', '', {}),
  mode: 'add',
  keys: keys.filter(
    (k, i) => i === 0 || k.clip !== keys[i - 1]!.clip || k.frame !== keys[i - 1]!.frame
  ),
});

/** A right hand closed around a handle, as the core's starters hold one. */
function rightHand(curl: Turn, wrist: Turn): GripPose {
  const pose: GripPose = { rightHand: wrist };
  ['Index', 'Middle', 'Ring', 'Little'].forEach((finger, f) =>
    ['Proximal', 'Intermediate', 'Distal'].forEach((joint, j) => {
      pose[`right${finger}${joint}`] = [0, 0, curl[j]! + f * 3];
    })
  );
  pose.rightThumbMetacarpal = [18, -24, 10];
  pose.rightThumbProximal = [0, -16, Math.round(curl[0] / 3)];
  pose.rightThumbDistal = [0, -12, Math.round(curl[1] / 4)];
  return pose;
}

const grip = (pose: GripPose): RecipeLayer => ({
  ...layer('l-2', 'grip', 'rightHand', { easeIn: 4, easeOut: 4 }),
  mode: 'replace',
  pose,
});

function slashCombo(r: RecipeFile) {
  r.layers = [
    keyed([
      key(r, 0, 0.15, 'smooth', { chest: [0, 0, 0] }),
      key(r, 0, 0.5, 'out', { chest: [8, -14, 0] }),
      key(r, 1, 0.1, 'smooth', { chest: [0, 0, 0] }),
      key(r, 1, 0.5, 'out', { chest: [6, 16, 0] }),
      key(r, 2, 0.3, 'smooth', { chest: [0, 0, 0] }),
    ]),
    grip(rightHand([62, 74, 48], [0, 0, 6])),
  ];
}

function scytheCharge(r: RecipeFile) {
  const wound = at(r, 0, 0.3);
  const held = Math.max(at(r, 0, 0.55) - wound, 1);
  const ease = Math.min(4, held);
  r.time = [
    {
      kind: 'hold-over',
      clip: 0,
      at: wound,
      frames: held,
      breathe: 0.3,
      easeIn: ease,
      easeOut: ease,
    },
  ];
  const right = rightHand([58, 70, 44], [0, 8, 0]);
  r.layers = [
    keyed([
      key(r, 0, 0.1, 'smooth', { spine: [0, 0, 0] }),
      key(r, 0, 0.3, 'out', { spine: [-10, 20, 0] }),
      key(r, 0, 0.7, 'in', { spine: [8, -24, 0] }),
      key(r, 1, 0.2, 'smooth', { spine: [0, 0, 0] }),
    ]),
    grip({ ...mirrorGrip(right), ...right }),
  ];
}

export const COMBOS: Combo[] = [
  {
    key: 'slash-combo',
    name: 'Slash combo',
    about:
      'Two sword cuts blended into a guard, the chest leaning into each cut, the sword held in the right hand.',
    clips: SLASH_CLIPS,
    mirror: false,
    shape: slashCombo,
  },
  {
    key: 'slash-combo-left',
    name: 'Slash combo, left handed',
    about: 'The slash combo mirrored: the left hand holds the sword and leads each cut.',
    clips: SLASH_CLIPS,
    mirror: true,
    shape: slashCombo,
  },
  {
    key: 'scythe-charge',
    name: 'Scythe charge',
    about:
      'Winds a two handed swing back, holds the charge, then lets it go into an idle; both hands on the scythe.',
    clips: SCYTHE_CLIPS,
    mirror: false,
    shape: scytheCharge,
  },
  {
    key: 'scythe-charge-left',
    name: 'Scythe charge, left handed',
    about: 'The scythe charge mirrored: it winds back and swings to the other side.',
    clips: SCYTHE_CLIPS,
    mirror: true,
    shape: scytheCharge,
  },
];

function clipOf(join: Join, clips: readonly AnimationClip[]): RecipeBase {
  const found = join.names.map(n => clipNamed(clips, n)).find(Boolean);
  if (found) return baseOf(found);
  return {
    ...namedBase(join.names[0] ?? ''),
    frames: join.frames,
    length: join.frames / REST_FPS,
  };
}

const kept = (c: RecipeBase) => (c.trim ? c.trim[1] - c.trim[0] : c.frames);

/** `combo` as a recipe file on what `clips` holds, made now. */
export function comboFile(combo: Combo, clips: readonly AnimationClip[]): RecipeFile {
  const found = combo.clips.map(j => clipOf(j, clips));
  const joined = found.slice(1).map((c, i) => ({
    ...c,
    blend: {
      frames: Math.min(
        combo.clips[i + 1]!.blend,
        Math.floor(Math.min(kept(found[i]!), kept(c)) / 2)
      ),
      curve: 'smooth' as const,
      lineUp: 'hips-and-facing' as const,
    },
  }));
  const r: RecipeFile = {
    ...newRecipe(combo.name, found[0]!),
    version: 2,
    joined,
    mirror: combo.mirror,
  };
  combo.shape(r);
  return r;
}
