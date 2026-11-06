/**
 * Recipe bakes in mock mode: Apply runs a fake headless bake that steps
 * through its frames on a timer, telling the view through the recipe bake
 * changed event, and ends with made up numbers: each reach within its 2 cm,
 * a small stretch and a foot slide past its limit. Cancel stops it with nothing staged. A
 * recipe with open blanks or no changes is refused as the core refuses it.
 * A finished bake carries its review as the core works it out, and Add
 * makes it the model's next version (refused once out of date). With
 * `?library=demo`, Frostveil's lantern recipe opens with a finished bake,
 * for headless shots of the result; `?agents=recipe` also has an agent ask
 * to add it.
 */
import type {
  RecipeBakeRecord,
  RecipeBakeRequest,
  RecipeBakeResult,
  RecipeBakeReview,
  RecipeDoc,
  RecipeReviewLine,
} from '../client';
import { events } from '../generated/bindings';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { currentAsks } from './recipe-bake-asks';

interface BakeContext {
  /** The recipe as `recipe_get` reads it; fails for one that is gone. */
  doc: (project: string, id: string) => RecipeDoc;
  /** The asset's name and newest version, or null when the project has no such asset. */
  asset: (project: string, asset: string) => { name: string; version: number } | null;
  /** Adds the staged model as the asset's next version; its number. */
  addVersion: (project: string, asset: string) => number;
  /** Serves the staged model as the asset's newest file does. */
  stage: (project: string, asset: string, file: string) => void;
  /** The bake the demo opens with: onto the first rigged model there is. */
  demoModel: () => string;
  /**
   * A clip the bundled stand-in file of `asset` has, for the review to play:
   * a mock bake cannot write the recipe's own clip into it.
   */
  standInClip: (asset: string) => string | null;
  state: string | null;
}

/** The core's words for a bake that is out of date. */
const OUT_OF_DATE = 'Not added: this bake is out of date.';
const BAKE_CHANGED = 'Not added: the bake changed since you looked at it.';

const round = (n: number) => Math.round(n * 10) / 10;

/** The review the core works out of a result (see bake/review.rs). */
export function reviewOf(result: RecipeBakeResult): RecipeBakeReview {
  const lines: RecipeReviewLine[] = [];
  for (const r of result.reaches) {
    const foot = r.bone.endsWith('Foot');
    const about = `${r.bone} (${r.layer})`;
    lines.push({
      id: `gap-${r.layer}`,
      label: foot ? 'Foot gap' : 'Hand gap',
      about,
      value: r.gapCm == null ? 'not measured' : `${r.gapCm.toFixed(1)} cm`,
      limit: '2 cm',
      measured: r.gapCm,
      max: 2,
      unit: 'cm',
      status: r.reached ? 'pass' : 'warn',
      note:
        r.gapCm == null
          ? 'Its end frame was not measured, so it counts as short.'
          : `At frame ${r.frame}, the reach's end.`,
    });
    const joint = foot ? 'Knee' : 'Elbow';
    lines.push({
      id: `flip-${r.layer}`,
      label: joint,
      about,
      value: r.flipFrame == null ? 'bends as the clip has it' : `flips at frame ${r.flipFrame}`,
      limit: 'no flip',
      measured: null,
      max: null,
      unit: null,
      status: r.flipFrame == null ? 'pass' : 'warn',
      note:
        r.flipFrame == null
          ? null
          : `The ${joint.toLowerCase()} bends the other way than the clip has it from there.`,
    });
  }
  const slideMax = round((result.baseSlideCm ?? 0) + 1.5);
  lines.push({
    id: 'feet',
    label: 'Foot slide',
    about: null,
    value: `${(result.slideCm ?? 0).toFixed(1)} cm${result.slideFoot ? ` (${result.slideFoot})` : ''}`,
    limit: `${slideMax.toFixed(1)} cm`,
    measured: result.slideCm,
    max: slideMax,
    unit: 'cm',
    status: (result.slideCm ?? 0) > slideMax ? 'warn' : 'pass',
    note: `The base clip slides ${(result.baseSlideCm ?? 0).toFixed(1)} cm on this model; up to 1.5 cm more passes.`,
  });
  const stretchMax = result.baseStretchMm == null ? null : round(result.baseStretchMm + 5);
  lines.push({
    id: 'stretch',
    label: 'Rig stretch',
    about: null,
    value: result.stretchMm == null ? 'not measured' : `${result.stretchMm.toFixed(1)} mm`,
    limit: stretchMax == null ? 'none' : `${stretchMax.toFixed(1)} mm`,
    measured: result.stretchMm,
    max: stretchMax,
    unit: 'mm',
    status:
      result.stretchMm == null
        ? 'not-measured'
        : stretchMax != null && result.stretchMm > stretchMax
          ? 'warn'
          : 'pass',
    note:
      result.stretchMm == null
        ? result.stretchNote
        : result.baseStretchMm == null
          ? "The base clip's stretch was not measured, so nothing to hold it to."
          : `The base clip stretches ${result.baseStretchMm.toFixed(1)} mm; up to 5 mm more passes.`,
  });
  return { lines, warns: lines.filter(l => l.status === 'warn').length };
}

/** Frames the fake bake steps through, and milliseconds per step. */
const STEPS = 8;
const STEP_MS = 450;

const snake = (name: string) =>
  name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_|_$/g, '') || 'recipe';

function resultOf(
  doc: RecipeDoc,
  asset: string,
  onto: { name: string; version: number },
  standIn: string | null
): RecipeBakeResult {
  const reaches = doc.recipe.layers.filter(l => l.kind === 'reach');
  return {
    recipe: doc.id,
    recipeName: doc.recipe.name,
    recipeVersion: doc.version,
    motionVersion: doc.motionVersion,
    baseClip: doc.base.clip ?? '',
    baseHash: doc.recipe.base.hash,
    recipeBaseHash: null,
    mixamo: doc.mixamo,
    asset,
    assetName: onto.name,
    fromVersion: onto.version,
    clip: standIn ?? snake(doc.recipe.name),
    frames: doc.recipe.base.frames,
    file: `/cache/recipes/${doc.id}/staged.glb`,
    facing: [0, 0, 1],
    // The planted left foot slides past its limit, so a review has a warn to show.
    slideCm: 2.6,
    slideFrames: [14, 22],
    slideFoot: 'leftFoot',
    baseSlideCm: 0.8,
    stretchMm: 3.4,
    stretchFrame: 12,
    baseStretchMm: 2.9,
    stretchNote: null,
    reaches: reaches.map((l, i) => ({
      layer: l.id,
      bone: l.bone,
      gapCm: 0.3 + i * 0.4,
      frame: Math.floor((l.settings.end ?? 1) * doc.recipe.base.frames),
      reached: true,
      flipFrame: null,
    })),
    blenderSeconds: 2.1,
    seconds: 2.6,
    warnings: doc.base.warning ? [doc.base.warning] : [],
  };
}

export function createRecipeBakeHandlers(ctx: BakeContext) {
  const bakes = new Map<string, RecipeBakeRecord>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const keyOf = (project: unknown, recipe: unknown) => `${String(project)}/${String(recipe)}`;
  const reviewed = (record: RecipeBakeRecord): RecipeBakeRecord =>
    record.state === 'done' && record.result
      ? { ...record, review: reviewOf(record.result) }
      : record;
  const put = (record: RecipeBakeRecord) => {
    const next = reviewed({ ...record, updatedMs: clock.nowMs() });
    // A new bake of the recipe ends what an agent's add waited for.
    if (next.state === 'running') currentAsks()?.close(next.project, next.recipe);
    if (next.state === 'done' && next.result) ctx.stage(next.project, next.asset, next.result.file);
    bakes.set(keyOf(next.project, next.recipe), next);
    // Tests that do not mock events have no listener to tell.
    events.recipeBakeChanged.emit({ record: next }).catch(() => {});
    return next;
  };
  let seeded = ctx.state !== 'demo';
  const seed = () => {
    if (seeded) return;
    seeded = true;
    try {
      const doc = ctx.doc('frostveil', 'reach-for-the-lantern');
      const now = clock.nowMs();
      const asset = ctx.demoModel();
      const record: RecipeBakeRecord = {
        project: 'frostveil',
        recipe: doc.id,
        recipeName: doc.recipe.name,
        asset,
        state: 'done',
        startedBy: 'person',
        progress: null,
        error: null,
        startedMs: now - 4000,
        updatedMs: now - 1400,
        finishedMs: now - 1400,
        result: resultOf(
          doc,
          asset,
          ctx.asset('frostveil', asset) ?? { name: 'Elsheria', version: 1 },
          ctx.standInClip(asset)
        ),
      };
      const done = reviewed(record);
      bakes.set(keyOf('frostveil', doc.id), done);
      if (done.result) ctx.stage('frostveil', asset, done.result.file);
      if (currentAsks()?.demo) currentAsks()?.ask({ ...done, startedBy: 'agent' });
    } catch {
      // No demo recipe to show a bake for.
    }
  };
  const step = (key: string, done: number) => {
    const record = bakes.get(key);
    if (!record || record.state !== 'running') return;
    if (done >= STEPS) {
      timers.delete(key);
      const doc = ctx.doc(record.project, record.recipe);
      const onto = ctx.asset(record.project, record.asset) ?? { name: record.asset, version: 1 };
      put({
        ...record,
        state: 'done',
        progress: null,
        finishedMs: clock.nowMs(),
        result: resultOf(doc, record.asset, onto, ctx.standInClip(record.asset)),
      });
      return;
    }
    const frames = ctx.doc(record.project, record.recipe).recipe.base.frames;
    const frame = Math.round(((done + 1) / STEPS) * frames);
    put({
      ...record,
      progress: { label: `Solving frame ${frame} of ${frames}`, done: done + 1, total: STEPS },
    });
    timers.set(
      key,
      setTimeout(() => step(key, done + 1), STEP_MS)
    );
  };

  return {
    recipe_bake_get: ({ project, recipe }: Args) => {
      seed();
      return bakes.get(keyOf(project, recipe)) ?? null;
    },
    recipe_bake_start: ({ request }: Args) => {
      seed();
      const r = request as RecipeBakeRequest;
      const doc = ctx.doc(r.project, r.recipe);
      if (r.expected != null && r.expected !== doc.version)
        fail('moved', `${doc.recipe.name} changed since it was read, so it was not baked.`);
      const open = doc.recipe.blanks.filter(b => (b.kind === 'prop' ? !b.prop : !b.point));
      if (open.length)
        fail(
          'invalid',
          `Fill the open blanks of ${doc.recipe.name} first: ${open.map(b => b.id).join(', ')}. A bake needs every target.`
        );
      if (!doc.recipe.layers.length)
        fail(
          'invalid',
          `${doc.recipe.name} has no changes yet. Add a reach, nudge, look at or wave first.`
        );
      const key = keyOf(r.project, r.recipe);
      if (bakes.get(key)?.state === 'running')
        fail('invalid', `${doc.recipe.name} is baking already. Wait for it, or cancel it first.`);
      const now = clock.nowMs();
      const record = put({
        project: r.project,
        recipe: r.recipe,
        recipeName: doc.recipe.name,
        asset: r.asset,
        state: 'running',
        startedBy: 'person',
        progress: null,
        error: null,
        startedMs: now,
        updatedMs: now,
        finishedMs: null,
        result: null,
      });
      timers.set(
        key,
        setTimeout(() => step(key, 0), STEP_MS)
      );
      return record;
    },
    recipe_bake_add: ({ project, recipe, finishedMs }: Args) => {
      seed();
      const key = keyOf(project, recipe);
      const record = bakes.get(key);
      if (!record) fail('invalid', 'This recipe has no bake to add. Apply it first.');
      if (record.state === 'done' && record.finishedMs !== (finishedMs ?? null))
        fail(
          'bake-changed',
          `${BAKE_CHANGED} ${record.recipeName} was baked again since. Look at the new bake, then add it.`
        );
      if (record.state === 'added')
        fail('invalid', `This bake was added already, as v${record.addedVersion ?? 0}.`);
      if (record.state !== 'done' || !record.result)
        fail('invalid', 'The last bake did not finish, so there is nothing to add. Bake it again.');
      const doc = ctx.doc(record.project, record.recipe);
      const unchanged = record.result.motionVersion
        ? doc.motionVersion === record.result.motionVersion
        : doc.version === record.result.recipeVersion;
      if (!unchanged)
        fail('invalid', `${OUT_OF_DATE} The recipe changed since this bake. Bake it again.`);
      const onto = ctx.asset(record.project, record.asset);
      if (onto && onto.version !== record.result.fromVersion)
        fail(
          'invalid',
          `${OUT_OF_DATE} ${onto.name} is at v${onto.version} now, and this bake went onto v${record.result.fromVersion}. Bake it again.`
        );
      const version = ctx.addVersion(record.project, record.asset);
      currentAsks()?.close(record.project, record.recipe);
      return put({ ...record, state: 'added', review: null, addedVersion: version });
    },
    recipe_bake_cancel: ({ project, recipe }: Args) => {
      const key = keyOf(project, recipe);
      const record = bakes.get(key);
      if (record?.state !== 'running') fail('invalid', 'This recipe is not baking.');
      clearTimeout(timers.get(key));
      timers.delete(key);
      put({ ...record, state: 'cancelled', progress: null, finishedMs: clock.nowMs() });
      return null;
    },
  } as Record<string, (args: Args) => unknown>;
}
