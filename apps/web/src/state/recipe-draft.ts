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
  RecipeBlank,
  RecipeFile,
  RecipeLayer,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { boneWords, bonesFor } from './recipe-bones';

/** What the editor changes: the changes and the blanks, each sent whole. */
export interface RecipeDraft {
  layers: RecipeLayer[];
  blanks: RecipeBlank[];
}

export const KIND_WORDS: Record<LayerKind, string> = {
  reach: 'Reach',
  nudge: 'Nudge',
  'look-at': 'Look at',
  wave: 'Wave',
};

export const KIND_ABOUT: Record<LayerKind, string> = {
  reach: 'A hand or foot reaches a blank',
  nudge: "Turns or moves one bone's pose",
  'look-at': 'The head or neck follows a blank',
  wave: 'An arm swings, laid over the clip',
};

export const BLANK_WORDS: Record<BlankKind, string> = { prop: 'Prop', point: 'Point' };

export const draftOf = (recipe: RecipeFile): RecipeDraft => ({
  layers: recipe.layers,
  blanks: recipe.blanks,
});

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

/** A new change of `kind` on its kind's first bone, over the middle half of the clip. */
export function newLayer(
  kind: LayerKind,
  layers: readonly RecipeLayer[],
  blanks: readonly RecipeBlank[]
): RecipeLayer {
  const bone = kind === 'nudge' ? 'spine' : (bonesFor(kind)[0] ?? 'hips');
  const preferred = kind === 'reach' || kind === 'wave' ? 'rightHand' : bone;
  return {
    id: freeId(
      'l',
      layers.map(l => l.id)
    ),
    kind,
    bone: bonesFor(kind).includes(preferred) ? preferred : bone,
    settings: settingsFor(kind, EMPTY, blanks),
  };
}

/** `layer` as a change of `kind`: the bone stays when the kind may move it. */
export function withKind(
  layer: RecipeLayer,
  kind: LayerKind,
  blanks: readonly RecipeBlank[]
): RecipeLayer {
  const bones = bonesFor(kind);
  return {
    ...layer,
    kind,
    bone: bones.includes(layer.bone) ? layer.bone : (bones[0] ?? layer.bone),
    settings: settingsFor(kind, layer.settings, blanks),
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

/** "Reach · Right hand → lantern · frames 24 to 72". */
export function layerLine(layer: RecipeLayer, frames: number): string {
  const s = layer.settings;
  const aim = s.target ? ` → ${s.target}` : '';
  const span = `frames ${frameOf(num(s.start), frames)} to ${frameOf(num(s.end, 1), frames)}`;
  return [`${KIND_WORDS[layer.kind]} · ${boneWords(layer.bone)}${aim}`, span].join(' · ');
}

/** What fills a blank, in words: the prop's name, the point, or open. */
export function blankFill(blank: RecipeBlank, assetName: (id: string) => string | null): string {
  if (blank.kind === 'prop')
    return blank.prop ? (assetName(blank.prop) ?? blank.prop) : 'open, pick a prop';
  if (!blank.point) return 'open, set a point';
  return blank.point.map(v => `${num(v).toFixed(2)}`).join(', ') + ' m';
}
