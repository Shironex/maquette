/**
 * The mock animation library (`?library=demo`): clips for Frostveil's
 * Elsheria, the swordswoman. Sword and shield and magic packs, single
 * dodges, a Tripo take and a custom one. Every clip plays the bundled
 * `clip_skeleton.glb`, a skeleton only clip the core's reader wrote from
 * its own test rig.
 */
import type { AnimationClip, RestBone, RootMotion, SourceRig } from '../client';
import { clock } from '@/lib/clock';

const DAY = 86_400_000;

/** The bundled clip under `assets/mock/models`. */
export const CLIP_MODEL = 'clip_skeleton.glb';

type Point = [number, number, number];

/** The test rig's rest pose: name, parent, head, tail, in glTF axes. */
const REST: readonly [string, string | null, Point, Point][] = [
  ['Hips', null, [0, 1, 0], [0, 1.1, 0]],
  ['Spine', 'Hips', [0, 1.1, 0], [0, 1.2, 0]],
  ['Spine1', 'Spine', [0, 1.2, 0], [0, 1.3, 0]],
  ['Spine2', 'Spine1', [0, 1.3, 0], [0, 1.41, 0]],
  ['Neck', 'Spine2', [0, 1.42, 0], [0, 1.52, 0]],
  ['Head', 'Neck', [0, 1.52, 0], [0, 1.62, 0]],
  ['LeftShoulder', 'Spine2', [0.05, 1.4, 0], [0.15, 1.4, 0]],
  ['LeftArm', 'LeftShoulder', [0.15, 1.4, 0], [0.42, 1.4, 0]],
  ['LeftForeArm', 'LeftArm', [0.42, 1.4, 0], [0.66, 1.4, 0]],
  ['LeftHand', 'LeftForeArm', [0.66, 1.4, 0], [0.9, 1.4, 0]],
  ['RightShoulder', 'Spine2', [-0.05, 1.4, 0], [-0.15, 1.4, 0]],
  ['RightArm', 'RightShoulder', [-0.15, 1.4, 0], [-0.42, 1.4, 0]],
  ['RightForeArm', 'RightArm', [-0.42, 1.4, 0], [-0.66, 1.4, 0]],
  ['RightHand', 'RightForeArm', [-0.66, 1.4, 0], [-0.9, 1.4, 0]],
  ['LeftUpLeg', 'Hips', [0.1, 0.98, 0], [0.1, 0.52, 0]],
  ['LeftLeg', 'LeftUpLeg', [0.1, 0.52, 0], [0.1, 0.08, 0]],
  ['LeftFoot', 'LeftLeg', [0.1, 0.08, 0], [0.1, 0.02, 0.12]],
  ['LeftToeBase', 'LeftFoot', [0.1, 0.02, 0.12], [0.1, 0.02, 0.25]],
  ['RightUpLeg', 'Hips', [-0.1, 0.98, 0], [-0.1, 0.52, 0]],
  ['RightLeg', 'RightUpLeg', [-0.1, 0.52, 0], [-0.1, 0.08, 0]],
  ['RightFoot', 'RightLeg', [-0.1, 0.08, 0], [-0.1, 0.02, 0.12]],
  ['RightToeBase', 'RightFoot', [-0.1, 0.02, 0.12], [-0.1, 0.02, 0.25]],
];

/** Tripo's names for the same bones, in the same order. */
const TRIPO = [
  'Hip',
  'Waist',
  'Spine01',
  'Spine02',
  'NeckTwist01',
  'Head',
  'L_Clavicle',
  'L_Upperarm',
  'L_Forearm',
  'L_Hand',
  'R_Clavicle',
  'R_Upperarm',
  'R_Forearm',
  'R_Hand',
  'L_Thigh',
  'L_Calf',
  'L_Foot',
  'L_ToeBase',
  'R_Thigh',
  'R_Calf',
  'R_Foot',
  'R_ToeBase',
];

export function restBones(rig: SourceRig): RestBone[] {
  const name = (n: string) => {
    if (rig === 'tripo') return TRIPO[REST.findIndex(r => r[0] === n)] ?? n;
    return rig === 'custom' ? `heroine_${n.toLowerCase()}` : `mixamorig:${n}`;
  };
  return REST.map(([bone, parent, head, tail]) => ({
    name: name(bone),
    parent: parent === null ? null : name(parent),
    head,
    tail,
  }));
}

interface Seed {
  name: string;
  pack: string;
  tags: string[];
  frames: number;
  fps?: number;
  root?: RootMotion;
  loops?: boolean;
  rig?: SourceRig;
  kb: number;
  /** Days since it was added. */
  days: number;
  /** The file it came from, when not `<name>.fbx`. */
  from?: string;
}

const SWORD = 'Pro Sword and Shield Pack';
const MAGIC = 'Pro Magic Pack';

const SEEDS: readonly Seed[] = [
  {
    name: 'Standing Dodge Left',
    pack: 'Singles',
    tags: ['dodge', 'combat'],
    frames: 38,
    root: { metres: 0.92, direction: 'left' },
    kb: 46,
    days: 2,
  },
  {
    name: 'Run To Rolling',
    pack: 'Singles',
    tags: ['dodge', 'locomotion'],
    frames: 52,
    root: { metres: 2.26, direction: 'forward' },
    kb: 61,
    days: 2,
  },
  {
    name: 'Standing Dodge Right',
    pack: 'Singles',
    tags: ['dodge', 'combat'],
    frames: 38,
    root: { metres: 0.9, direction: 'right' },
    kb: 46,
    days: 2,
  },
  {
    name: 'Standing Dodge Backward',
    pack: 'Singles',
    tags: ['dodge', 'combat'],
    frames: 34,
    root: { metres: 0.74, direction: 'back' },
    kb: 42,
    days: 2,
  },
  {
    name: 'Sword And Shield Idle',
    pack: SWORD,
    tags: ['combat', 'idle'],
    frames: 120,
    loops: true,
    kb: 118,
    days: 9,
  },
  {
    name: 'Sword And Shield Run',
    pack: SWORD,
    tags: ['combat', 'locomotion'],
    frames: 22,
    root: { metres: 3.1, direction: 'forward' },
    loops: true,
    kb: 31,
    days: 9,
  },
  { name: 'Sword And Shield Slash', pack: SWORD, tags: ['combat'], frames: 48, kb: 58, days: 9 },
  { name: 'Sword And Shield Block', pack: SWORD, tags: ['combat'], frames: 30, kb: 37, days: 9 },
  { name: 'Sword And Shield Crouch', pack: SWORD, tags: ['combat'], frames: 44, kb: 52, days: 9 },
  {
    name: 'Sword And Shield Impact',
    pack: SWORD,
    tags: ['combat', 'hit react'],
    frames: 26,
    kb: 33,
    days: 9,
  },
  {
    name: 'Standing 1H Magic Attack 01',
    pack: MAGIC,
    tags: ['magic'],
    frames: 64,
    kb: 74,
    days: 6,
  },
  {
    name: 'Standing 2H Magic Area Attack 02',
    pack: MAGIC,
    tags: ['magic'],
    frames: 80,
    kb: 92,
    days: 6,
  },
  { name: 'Standing 1H Cast Spell 01', pack: MAGIC, tags: ['magic'], frames: 56, kb: 66, days: 6 },
  {
    name: 'Magic Idle',
    pack: MAGIC,
    tags: ['magic', 'idle'],
    frames: 90,
    loops: true,
    kb: 98,
    days: 6,
  },
  {
    name: 'Duelist lunge',
    pack: 'Tripo animations',
    tags: ['combat'],
    frames: 36,
    fps: 24,
    root: { metres: 1.1, direction: 'forward' },
    rig: 'tripo',
    kb: 41,
    days: 4,
    from: 'duelist.glb',
  },
  {
    name: 'Heroine walk, take 2',
    pack: 'Elsheria takes',
    tags: ['locomotion'],
    frames: 32,
    root: { metres: 1.4, direction: 'forward' },
    loops: true,
    rig: 'custom',
    kb: 38,
    days: 12,
    from: 'heroine_walk_take2.fbx',
  },
];

const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

/** A clip as the library would hold it, for the mock's seeds and imports. */
export function mockClip(seed: Omit<Seed, 'days'> & { id?: string; at: string }): AnimationClip {
  const fps = seed.fps ?? 30;
  const rig = seed.rig ?? 'mixamo';
  const zipped = seed.pack !== 'Singles' && rig === 'mixamo';
  return {
    version: 1,
    id: seed.id ?? slug(seed.name),
    name: seed.name,
    pack: seed.pack,
    tags: seed.tags,
    rig: { kind: rig, bones: restBones(rig), hipsHeight: 1 },
    frames: seed.frames,
    fps,
    length: seed.frames / fps,
    rootMotion: seed.root ?? null,
    loops: seed.loops ?? false,
    bytes: seed.kb * 1000,
    file: 'clip.glb',
    origin: {
      source: rig === 'tripo' ? 'tripo' : 'import',
      file: seed.from ?? `${seed.name}.fbx`,
      archive: zipped ? `${seed.pack}.zip` : null,
      kept: null,
      project: null,
      asset: null,
      version: null,
    },
    hash: slug(seed.name),
    revision: 1,
    createdAt: seed.at,
    updatedAt: seed.at,
  };
}

export function animationClips(): AnimationClip[] {
  const now = clock.nowMs();
  return SEEDS.map((s, i) =>
    mockClip({ ...s, at: new Date(now - s.days * DAY - i * 60_000).toISOString() })
  );
}

/** What the mock reads from a pack zip, by its name: clip names, with a pose file left out. */
export const PACK_CLIPS: Record<string, { clips: string[]; tags: string[]; still?: string }> = {
  [SWORD]: {
    clips: [
      'Sword And Shield Idle',
      'Sword And Shield Kick',
      'Sword And Shield Turn',
      'Sword And Shield Jump',
    ],
    tags: ['combat'],
  },
  [MAGIC]: {
    clips: ['Standing 1H Magic Attack 02', 'Standing 2H Cast Spell 01', 'Magic Area Heal'],
    tags: ['magic'],
    still: 'magic_pose.fbx',
  },
};
