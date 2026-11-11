/**
 * A recipe's changes and blanks while the Recipes tab edits them: new
 * entries with settings their kind reads, a kind switch that keeps what it
 * can, and the words each change and blank shows. Pure helpers; the store
 * is use-recipe-editor-store.ts and the checks recipe-check.ts.
 */
import type {
  BlankKind,
  LayerKind,
  LayerSettings,
  RecipeBase,
  RecipeBlank,
  RecipeFile,
  RecipeLayer,
  TimeEdit,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { counted } from '@/lib/format';
import { boneWords, bonesFor } from './recipe-bones';
import { pinSettings } from './recipe-pins';
import { emptyKeys } from './recipe-key-edits';

/** What the editor changes, each list sent whole when it changed. */
export interface RecipeDraft {
  layers: RecipeLayer[];
  blanks: RecipeBlank[];
  /** Every clip, the base first, each with its trim and (after the base) its blend. */
  clips: RecipeBase[];
  /** For each clip, the index of the file's clip it keeps; null for one newly picked. */
  kept: (number | null)[];
  /** On each clip's own frames, before its trim. */
  time: TimeEdit[];
  mirror: boolean;
}

export const KIND_WORDS: Record<LayerKind, string> = {
  reach: 'Reach',
  nudge: 'Nudge',
  'look-at': 'Look at',
  wave: 'Wave',
  pin: 'Pin',
  pose: 'Pose keys',
  grip: 'Grip',
};

export const KIND_ABOUT: Record<LayerKind, string> = {
  reach: 'A hand or foot reaches a blank',
  nudge: "Turns or moves one bone's pose",
  'look-at': 'The head or neck follows a blank',
  wave: 'An arm swings, laid over the clip',
  pin: 'A hand or foot stays put',
  pose: 'Keys on the bones and frames you set',
  grip: 'A hand pose from the grip library',
};

/**
 * The kinds the Add change picker offers, in its order. A grip comes from
 * the grip library, so picking it opens the grip picker.
 */
export const ADD_KINDS: readonly LayerKind[] = [
  'reach',
  'nudge',
  'look-at',
  'wave',
  'pin',
  'pose',
  'grip',
];

/** The kinds a change can switch between in its form; pose keys and grips keep their own. */
export const SWITCH_KINDS: readonly LayerKind[] = ['reach', 'nudge', 'look-at', 'wave', 'pin'];

export const BLANK_WORDS: Record<BlankKind, string> = { prop: 'Prop', point: 'Point' };

export const draftOf = (recipe: RecipeFile): RecipeDraft => {
  const clips = [recipe.base, ...(recipe.joined ?? [])];
  return {
    layers: recipe.layers,
    blanks: recipe.blanks,
    clips,
    kept: clips.map((_, i) => i),
    time: recipe.time ?? [],
    mirror: Boolean(recipe.mirror),
  };
};

export const sameDraft = (a: RecipeDraft, b: RecipeDraft) => deepEqual(a, b);

/** A number the file holds as null (a NaN on the way out) reads as `or`. */
export const num = (v: number | null | undefined, or = 0) =>
  typeof v === 'number' && Number.isFinite(v) ? v : or;

const ID_CHARS = 'abcdefghijklmnopqrstuvwxyz0123456789';
/** Every id the editor handed out, so one never comes back after its entry is removed. */
const issued = new Set<string>();

/**
 * A fresh id for an entry made in the editor: `prefix-` and five random
 * letters and digits, at least one a letter. The core numbers the changes
 * it fills in (`l-3`), so the two never meet, and an agent picking the same
 * id at the same time is all but impossible (and merges as two additions).
 * No id is handed out twice.
 */
export function freeId(prefix: string, taken: readonly string[], random = Math.random): string {
  for (;;) {
    let tail = '';
    for (let i = 0; i < 5; i += 1) tail += ID_CHARS[Math.floor(random() * ID_CHARS.length)];
    const id = `${prefix}-${tail}`;
    if (/[a-z]/.test(tail) && !taken.includes(id) && !issued.has(id)) {
      issued.add(id);
      return id;
    }
  }
}

/** `settings` with only what `kind` reads, the rest filled with a starting value. */
export function settingsFor(
  kind: LayerKind,
  settings: LayerSettings,
  blanks: readonly RecipeBlank[]
): LayerSettings {
  const targeted = kind === 'reach' || kind === 'look-at';
  const nudge = kind === 'nudge';
  const wave = kind === 'wave';
  const target = settings.target ?? blanks[0]?.id ?? null;
  return {
    weight: settings.weight ?? 1,
    start: settings.start ?? 0,
    end: settings.end ?? 1,
    target: targeted ? target : null,
    rotation: nudge ? (settings.rotation ?? [0, 0, 0]) : null,
    offset: nudge ? settings.offset : null,
    cycles: wave ? (settings.cycles ?? 3) : null,
    amount: wave ? (settings.amount ?? 30) : null,
  };
}

const EMPTY: LayerSettings = {
  weight: 1,
  start: 0.25,
  end: 0.75,
  target: null,
  rotation: null,
  offset: null,
  cycles: null,
  amount: null,
};

/** A new change of `kind` on its kind's first bone, over the middle half of the clip; pose keys start with none. */
export function newLayer(
  kind: LayerKind,
  layers: readonly RecipeLayer[],
  blanks: readonly RecipeBlank[]
): RecipeLayer {
  if (kind === 'pose')
    return emptyKeys(
      freeId(
        'l',
        layers.map(l => l.id)
      )
    );
  const bone = kind === 'nudge' ? 'spine' : (bonesFor(kind)[0] ?? 'hips');
  const preferred =
    kind === 'reach' || kind === 'wave' ? 'rightHand' : kind === 'pin' ? 'leftFoot' : bone;
  const picked = bonesFor(kind).includes(preferred) ? preferred : bone;
  const settings = settingsFor(kind, EMPTY, blanks);
  return {
    id: freeId(
      'l',
      layers.map(l => l.id)
    ),
    kind,
    bone: picked,
    settings: kind === 'pin' ? pinSettings(settings, picked) : settings,
  };
}

/** `layer` as a change of `kind`: the bone stays when the kind may move it. */
export function withKind(
  layer: RecipeLayer,
  kind: LayerKind,
  blanks: readonly RecipeBlank[]
): RecipeLayer {
  const bones = bonesFor(kind);
  const bone = bones.includes(layer.bone) ? layer.bone : (bones[0] ?? layer.bone);
  const settings = settingsFor(kind, layer.settings, blanks);
  return {
    ...layer,
    kind,
    bone,
    settings: kind === 'pin' ? pinSettings(settings, bone) : settings,
  };
}

/** A new open blank of `kind`. */
export function newBlank(kind: BlankKind, blanks: readonly RecipeBlank[]): RecipeBlank {
  return {
    id: freeId(
      kind,
      blanks.map(b => b.id)
    ),
    label: kind === 'prop' ? 'The prop it aims at' : 'Where it aims',
    kind,
    prop: null,
    point: null,
  };
}

/** `blank` as a blank of `kind`; what it held goes, since the other kind cannot hold it. */
export const withBlankKind = (blank: RecipeBlank, kind: BlankKind): RecipeBlank =>
  blank.kind === kind ? blank : { ...blank, kind, prop: null, point: null };

/** The blanks without `id`, and changes that aimed at it aiming nowhere. */
export function withoutBlank(draft: RecipeDraft, id: string): RecipeDraft {
  return {
    ...draft,
    blanks: draft.blanks.filter(b => b.id !== id),
    layers: draft.layers.map(l =>
      l.settings.target === id ? { ...l, settings: { ...l.settings, target: null } } : l
    ),
  };
}

export const blankFilled = (b: RecipeBlank) => (b.kind === 'prop' ? Boolean(b.prop) : !!b.point);

/** The frame a share of a clip of `frames` frames lands on. */
export const frameOf = (share: number, frames: number) => Math.round(share * frames);

/** The share of the clip at `frame`, kept in 0 to 1. */
export const shareOf = (frame: number, frames: number) =>
  frames > 0 ? Math.min(Math.max(frame / frames, 0), 1) : 0;

/**
 * "Reach · Right hand → lantern · frames 24 to 72": the frames of the bake,
 * `frames` the clip's count or what turns a share into a frame of the
 * timeline.
 */
export function layerLine(
  layer: RecipeLayer,
  frames: number | ((share: number) => number)
): string {
  const s = layer.settings;
  if (layer.kind === 'pose') return poseKeysLine(layer);
  const aim = s.target ? ` → ${s.target}` : '';
  const at = typeof frames === 'number' ? (share: number) => frameOf(share, frames) : frames;
  const span = `frames ${at(num(s.start))} to ${at(num(s.end, 1))}`;
  if (layer.kind === 'grip') return [gripWords(layer) + aim, span].join(' · ');
  return [`${KIND_WORDS[layer.kind]} · ${boneWords(layer.bone)}${aim}`, span].join(' · ');
}

/** What fills a blank, in words: the prop's name, the point, or open. */
export function blankFill(blank: RecipeBlank, assetName: (id: string) => string | null): string {
  if (blank.kind === 'prop')
    return blank.prop ? (assetName(blank.prop) ?? blank.prop) : 'open, pick a prop';
  if (!blank.point) return 'open, set a point';
  return blank.point.map(v => `${num(v).toFixed(2)}`).join(', ') + ' m';
}

const HANDS_WORDS = { both: 'Both hands', right: 'Right hand', left: 'Left hand' } as const;

/**
 * "Grip · Both hands · scythe-staff-grip r1": the hands its pose turns and
 * the library grip it was copied from, when it has a link.
 */
export function gripWords(layer: RecipeLayer, name?: string | null): string {
  const bones = Object.keys(layer.pose ?? {});
  const left = bones.some(b => b.startsWith('left'));
  const right = bones.some(b => b.startsWith('right'));
  const hands = left && right ? 'both' : left ? 'left' : right ? 'right' : null;
  const from = layer.from ? `${name ?? layer.from.entry} r${layer.from.revision}` : null;
  return [KIND_WORDS.grip, hands ? HANDS_WORDS[hands] : boneWords(layer.bone), from]
    .filter(Boolean)
    .join(' · ');
}

/** "Pose keys · 3 keys on 2 bones · Add": a pose key change in one line. */
function poseKeysLine(layer: RecipeLayer): string {
  const keys = layer.keys ?? [];
  const bones = new Set(keys.flatMap(k => Object.keys(k.pose))).size;
  const count = keys.length
    ? `${counted(keys.length, 'key')} on ${counted(bones, 'bone')}`
    : 'no keys yet';
  return [KIND_WORDS.pose, count, layer.mode === 'replace' ? 'Replace' : 'Add'].join(' · ');
}
