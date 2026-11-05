/**
 * Recipe bakes in mock mode: Apply runs a fake headless bake that steps
 * through its frames on a timer, telling the view through the recipe bake
 * changed event, and ends with made up numbers: each reach within its 2 cm,
 * a small foot slide and stretch. Cancel stops it with nothing staged. A
 * recipe with open blanks or no changes is refused as the core refuses it.
 * With `?library=demo`, Frostveil's lantern recipe opens with a finished
 * bake, for headless shots of the result.
 */
import type { RecipeBakeRecord, RecipeBakeRequest, RecipeBakeResult, RecipeDoc } from '../client';
import { events } from '../generated/bindings';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';

interface BakeContext {
  /** The recipe as `recipe_get` reads it; fails for one that is gone. */
  doc: (project: string, id: string) => RecipeDoc;
  /** The asset's name, or null when the project has no such asset. */
  assetName: (project: string, asset: string) => string | null;
  state: string | null;
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

function resultOf(doc: RecipeDoc, asset: string, assetName: string): RecipeBakeResult {
  const reaches = doc.recipe.layers.filter(l => l.kind === 'reach');
  return {
    recipe: doc.id,
    recipeName: doc.recipe.name,
    recipeVersion: doc.version,
    baseClip: doc.base.clip ?? '',
    baseHash: doc.recipe.base.hash,
    recipeBaseHash: null,
    mixamo: doc.mixamo,
    asset,
    assetName,
    fromVersion: 1,
    clip: snake(doc.recipe.name),
    frames: doc.recipe.base.frames,
    file: `/cache/recipes/${doc.id}/staged.glb`,
    facing: [0, 0, 1],
    slideCm: 0.8,
    slideFrames: null,
    slideFoot: null,
    baseSlideCm: 0.8,
    stretchMm: 3.4,
    stretchFrame: 12,
    baseStretchMm: 2.9,
    stretchNote: null,
    reaches: reaches.map((l, i) => ({
      layer: l.id,
      bone: l.bone,
      gapCm: 0.3 + i * 0.4,
      frame: Math.floor((l.settings.end ?? 1) * (doc.recipe.base.frames - 1)),
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
  const put = (record: RecipeBakeRecord) => {
    const next = { ...record, updatedMs: clock.nowMs() };
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
      bakes.set(keyOf('frostveil', doc.id), {
        project: 'frostveil',
        recipe: doc.id,
        recipeName: doc.recipe.name,
        asset: 'elsheria',
        state: 'done',
        startedBy: 'person',
        progress: null,
        error: null,
        startedMs: now - 4000,
        updatedMs: now - 1400,
        finishedMs: now - 1400,
        result: resultOf(doc, 'elsheria', 'Elsheria'),
      });
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
      const name = ctx.assetName(record.project, record.asset) ?? record.asset;
      put({
        ...record,
        state: 'done',
        progress: null,
        finishedMs: clock.nowMs(),
        result: resultOf(doc, record.asset, name),
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
