/**
 * The core's recipe checks (crates/maquette-model/src/recipes/check.rs) run
 * on the draft as it is typed, so each change and blank says what to fix
 * next to it before Save. The core still checks the whole file on Save;
 * this only says it sooner.
 */
import type { RecipeBlank, RecipeLayer } from '@/ipc/client';
import { HUMAN_BONES, bonesFor } from './recipe-bones';
import { KIND_WORDS, num, type RecipeDraft } from './recipe-draft';
import { gripProblem } from './grip-problem';
import { poseKeysProblem } from './pose-keys-problem';
import { pinProblem } from './recipe-pins';
import { baseProblems } from './recipe-v2-check';

export const LIMITS = {
  layers: 32,
  blanks: 16,
  label: 120,
  id: 64,
  offset: 2,
  rotation: 180,
  point: 1000,
  amount: 90,
  cycles: 50,
} as const;

/** What to fix, by change id and blank id, by clip and time edit, plus lines about the whole recipe. */
export interface DraftProblems {
  layers: Record<string, string>;
  blanks: Record<string, string>;
  /** By clip: its trim, or the blend into it. */
  clips: Record<number, string>;
  /** By time edit. */
  time: Record<number, string>;
  general: string[];
}

/** What the checks read: the changes and blanks, and the clips and time edits when there are. */
export type CheckedDraft = Pick<RecipeDraft, 'layers' | 'blanks'> &
  Partial<Pick<RecipeDraft, 'clips' | 'time'>>;

/** The non-null entries of `list`, by index. */
const byIndex = (list: readonly (string | null)[]) =>
  Object.fromEntries(list.flatMap((p, i) => (p ? [[i, p] as const] : [])));

const ITEM_ID = /^[A-Za-z0-9_-]+$/;
const isItemId = (id: string) => id.length > 0 && id.length <= LIMITS.id && ITEM_ID.test(id);

const within = (values: readonly (number | null)[], limit: number) =>
  values.every(v => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= limit);

function blankProblem(blank: RecipeBlank): string | null {
  if (!isItemId(blank.id))
    return `"${blank.id}" is not a blank id. Use up to ${LIMITS.id} letters, digits, dashes and underscores.`;
  const label = blank.label.trim();
  if (!label) return `Give the blank ${blank.id} a name.`;
  if (label.length > LIMITS.label)
    return `Keep the blank ${blank.id} under ${LIMITS.label} characters.`;
  if (blank.kind === 'point' && blank.point && !within(blank.point, LIMITS.point))
    return `The blank ${blank.id} holds a point more than ${LIMITS.point} m away.`;
  return null;
}

function animatedProblem(
  blank: RecipeBlank,
  animated: { asset: string; name: string } | null
): string | null {
  if (!animated || blank.kind !== 'prop' || blank.prop !== animated.asset) return null;
  return `${animated.name} is the model this recipe animates. Pick another prop.`;
}

function settingsProblem(layer: RecipeLayer, blanks: readonly RecipeBlank[]): string | null {
  const s = layer.settings;
  const word = KIND_WORDS[layer.kind].toLowerCase();
  const weight = num(s.weight, Number.NaN);
  if (!(weight >= 0 && weight <= 1)) return 'The weight has to be 0 to 1.';
  const start = num(s.start, Number.NaN);
  const end = num(s.end, Number.NaN);
  if (!(start >= 0 && end <= 1 && start < end)) return 'It has to start before it ends.';
  switch (layer.kind) {
    case 'reach':
    case 'look-at': {
      if (!s.target) return `A ${word} needs a target: pick a blank, or add one.`;
      if (!blanks.some(b => b.id === s.target))
        return `It aims at the blank ${s.target}, which the recipe does not have.`;
      return null;
    }
    case 'nudge':
      if (!s.rotation && !s.offset) return 'A nudge needs a rotation, an offset or both.';
      if (s.rotation && !within(s.rotation, LIMITS.rotation))
        return `Keep the rotation within ${LIMITS.rotation} degrees each way.`;
      if (s.offset && !within(s.offset, LIMITS.offset))
        return `Keep the offset within ${LIMITS.offset} m each way.`;
      return null;
    case 'wave': {
      const cycles = num(s.cycles);
      const amount = num(s.amount);
      if (!(cycles > 0 && cycles <= LIMITS.cycles))
        return `Cycles: more than 0 and at most ${LIMITS.cycles}.`;
      if (!(amount > 0 && amount <= LIMITS.amount))
        return `Amount: more than 0 and at most ${LIMITS.amount} degrees.`;
      return null;
    }
    case 'pin':
      return pinProblem(layer, blanks);
    case 'grip':
      return gripProblem(layer, blanks);
    // Pose keys are checked before the settings, in layerProblem.
    case 'pose':
      return null;
  }
}

function layerProblem(
  layer: RecipeLayer,
  blanks: readonly RecipeBlank[],
  clips: CheckedDraft['clips']
): string | null {
  if (!isItemId(layer.id)) return `"${layer.id}" is not a change id.`;
  // Pose keys name their bones in each key, so the change itself has none.
  if (layer.kind === 'pose')
    return layer.bone
      ? 'Pose keys name their bones in each key.'
      : clips
        ? poseKeysProblem(layer, clips)
        : null;
  const bones = bonesFor(layer.kind);
  if (!bones.includes(layer.bone)) {
    return bones === HUMAN_BONES
      ? `"${layer.bone}" is not a humanoid bone.`
      : `A ${KIND_WORDS[layer.kind].toLowerCase()} moves ${bones.join(', ')}, not "${layer.bone}".`;
  }
  return settingsProblem(layer, blanks);
}

/**
 * Everything the core would refuse in `draft`, said where it is. With
 * `animated`, the model a bake of it goes onto, a prop blank holding that
 * model is marked too: the bake refuses it, so Save and Apply wait as they
 * do for an open blank.
 */
export function checkDraft(
  draft: CheckedDraft,
  animated: { asset: string; name: string } | null = null
): DraftProblems {
  const base = draft.clips ? baseProblems(draft.clips, draft.time ?? []) : null;
  const out: DraftProblems = {
    layers: {},
    blanks: {},
    clips: base ? byIndex(base.clips) : {},
    time: base ? byIndex(base.time) : {},
    general: base ? [...base.general] : [],
  };
  if (draft.layers.length > LIMITS.layers)
    out.general.push(`A recipe holds at most ${LIMITS.layers} changes.`);
  if (draft.blanks.length > LIMITS.blanks)
    out.general.push(`A recipe holds at most ${LIMITS.blanks} blanks.`);
  const seen = new Set<string>();
  for (const blank of draft.blanks) {
    const problem = seen.has(blank.id)
      ? `Two blanks are called ${blank.id}.`
      : (blankProblem(blank) ?? animatedProblem(blank, animated));
    if (problem) out.blanks[blank.id] = problem;
    seen.add(blank.id);
  }
  const ids = new Set<string>();
  for (const layer of draft.layers) {
    const problem = ids.has(layer.id)
      ? `Two changes have the id ${layer.id}.`
      : layerProblem(layer, draft.blanks, draft.clips);
    if (problem) out.layers[layer.id] = problem;
    ids.add(layer.id);
  }
  return out;
}

/** How many things wait to be fixed. */
export const problemCount = (p: DraftProblems) =>
  Object.keys(p.layers).length +
  Object.keys(p.blanks).length +
  Object.keys(p.clips).length +
  Object.keys(p.time).length +
  p.general.length;
