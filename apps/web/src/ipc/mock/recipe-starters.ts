/**
 * The starter recipes in mock mode, as the core builds them in: clip names
 * only (or the rest pose), never a hash or keys. A starter takes the first
 * library clip with one of its names, whatever its frames, else keeps the
 * first name and reads as missing its base. The combat starters, joined
 * clips with pose keys and a grip, are in recipe-combat-starters.ts.
 */
import type { AnimationClip, RecipeBlank, RecipeFile, RecipeStarter } from '../client';
import {
  baseOf,
  clipNamed,
  layer,
  matchBase,
  namedBase,
  newRecipe,
  restBase,
} from './recipe-files';
import { COMBOS, comboFile } from './recipe-combat-starters';

const IDLE = ['Idle', 'Breathing Idle', 'Standing Idle', 'Happy Idle'];
const WALK = ['Walking', 'Walk'];

interface Spec {
  key: string;
  name: string;
  about: string;
  /** Empty on the rest pose. */
  clips: string[];
  frames: number;
  blanks: RecipeBlank[];
  layers: RecipeFile['layers'];
}

const point = (id: string, label: string, at: [number, number, number]): RecipeBlank => ({
  id,
  label,
  kind: 'point',
  prop: null,
  point: at,
});

const SPECS: Spec[] = [
  {
    key: 'wave-hello',
    name: 'Wave hello',
    about: 'Raises the right arm and waves the forearm, on the rest pose.',
    clips: [],
    frames: 60,
    blanks: [],
    layers: [
      layer('l-1', 'nudge', 'rightUpperArm', { rotation: [0, 0, -60] }),
      layer('l-2', 'wave', 'rightLowerArm', { start: 0.15, end: 0.95, cycles: 3, amount: 30 }),
    ],
  },
  {
    key: 'look-at-point',
    name: 'Look at a point',
    about: 'Turns the head toward a point while the idle plays.',
    clips: IDLE,
    frames: 0,
    blanks: [point('spot', 'Where the head looks', [0.6, 1.6, 1.2])],
    layers: [layer('l-1', 'look-at', 'head', { start: 0.1, end: 0.9, target: 'spot' })],
  },
  {
    key: 'reach-for-prop',
    name: 'Reach for a prop',
    about: 'The right hand reaches a prop of the project while the idle plays.',
    clips: IDLE,
    frames: 0,
    blanks: [
      { id: 'prop', label: 'The prop the hand reaches', kind: 'prop', prop: null, point: null },
    ],
    layers: [layer('l-1', 'reach', 'rightHand', { start: 0.2, end: 0.8, target: 'prop' })],
  },
  {
    key: 'lean-in',
    name: 'Lean in',
    about: 'Leans the upper body forward over the idle, as if listening.',
    clips: IDLE,
    frames: 0,
    blanks: [],
    layers: [
      layer('l-1', 'nudge', 'spine', { start: 0.1, end: 0.9, rotation: [12, 0, 0] }),
      layer('l-2', 'nudge', 'head', { start: 0.1, end: 0.9, rotation: [-6, 0, 0] }),
    ],
  },
  {
    key: 'point-while-walking',
    name: 'Point while walking',
    about: 'The right hand points ahead and to the right during a walk.',
    clips: WALK,
    frames: 0,
    blanks: [point('spot', 'Where the hand points', [-0.4, 1.4, 0.8])],
    layers: [layer('l-1', 'reach', 'rightHand', { start: 0.25, end: 0.75, target: 'spot' })],
  },
];

export const STARTER_PREFIX = 'starter:';

const specOf = (id: string) => SPECS.find(s => `${STARTER_PREFIX}${s.key}` === id);

/** `id`'s recipe file on what `clips` holds, made now; none for an unknown starter. */
export function starterFile(id: string, clips: readonly AnimationClip[]): RecipeFile | null {
  const combo = COMBOS.find(c => `${STARTER_PREFIX}${c.key}` === id);
  if (combo) return comboFile(combo, clips);
  const spec = specOf(id);
  if (!spec) return null;
  const clip = spec.clips.map(n => clipNamed(clips, n)).find(Boolean);
  const base = spec.clips.length
    ? clip
      ? baseOf(clip)
      : namedBase(spec.clips[0] ?? '')
    : restBase(spec.frames);
  return { ...newRecipe(spec.name, base), blanks: spec.blanks, layers: spec.layers };
}

/** Every starter as `recipe_starter_list` answers it, the plain ones first. */
export function starterList(clips: readonly AnimationClip[]): RecipeStarter[] {
  const all = [
    ...SPECS.map(s => ({ ...s, names: s.clips })),
    ...COMBOS.map(c => ({ ...c, names: c.clips[0]?.names ?? [] })),
  ];
  return all.map(spec => {
    const id = `${STARTER_PREFIX}${spec.key}`;
    const file = starterFile(id, clips) as RecipeFile;
    const [base, clip] = matchBase(clips, file.base);
    const missing = (file.joined ?? [])
      .filter(c => matchBase(clips, c)[0].status === 'missing')
      .map(c => c.name);
    return {
      id,
      name: spec.name,
      about: spec.about,
      clips: spec.names,
      rest: spec.names.length === 0,
      blanks: file.blanks,
      layers: file.layers,
      baseName: clip?.name ?? file.base.name,
      base,
      mixamo: file.base.mixamo,
      ...(file.joined?.length ? { joined: file.joined.map(c => c.name) } : {}),
      ...(missing.length ? { missing } : {}),
      mirror: Boolean(file.mirror),
      ...(file.time?.length ? { time: file.time } : {}),
    };
  });
}
