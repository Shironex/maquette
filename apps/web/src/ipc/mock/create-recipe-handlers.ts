/**
 * Animation recipes in mock mode: each project's recipes kept in memory,
 * made from the mock clip library, with the core's checks that matter to
 * the view (a name, a free id, a known clip, a recipe that still exists).
 * Base clips match by name and frame count as the core matches them. Prop
 * blanks are read against the project as the core reads them: a prop that is
 * no longer an asset reads open with a warning, stays stored, and a write is
 * checked whole first. With `?library=demo`, Frostveil opens with two
 * recipes: one with a reach, a look and a wave on its first prop and a
 * point, and one whose base clip the library has with other keys and whose
 * torch prop is gone. The lantern recipe came from r1 of a library recipe
 * whose newest is r3 (see create-recipe-library-handlers.ts). A third joins
 * three clips with trims, a retime, a freeze and blends.
 */
import type {
  AnimationClip,
  GoneProp,
  ProjectManifest,
  RecipeBlank,
  RecipeChange,
  RecipeDoc,
  RecipeFile,
  RecipeSummary,
} from '../client';
import { clock } from '@/lib/clock';
import { checkDraft } from '@/state/recipe-check';
import { fail, type Args } from './backend-options';
import { createRecipeBakeHandlers } from './create-recipe-bake-handlers';
import { createRecipeLibraryHandlers } from './create-recipe-library-handlers';
import {
  baseOf,
  layer,
  matchBase,
  motionVersionOf,
  newRecipe,
  recipeMixamo,
  restBase,
  versionOf,
} from './recipe-files';
import { checkSize, followClips, v2Problem, withVersion } from './recipe-v2';
import { slugOf } from './plan-store';
import { REACH_UID } from './recipe-library-fixtures';

interface RecipeContext {
  clips: () => readonly AnimationClip[];
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  alias: (path: string, from: string) => void;
  root: () => string;
  state: string | null;
}

/** The bundled clay figure's own clips; a mock bake onto it plays its wave. */
const STAND_IN_CLIPS: Record<string, string> = { 'clay-figure': 'Wave' };

const fileOf = (id: string) => `recipes/${id}.recipe.json`;

/**
 * Frostveil's demo recipes: a reach, a look and a wave on its first prop and
 * a point, and one whose clip came back with new keys and whose torch is gone.
 */
function demoRecipes(
  clips: readonly AnimationClip[],
  prop: string | null
): Map<string, RecipeFile> {
  const out = new Map<string, RecipeFile>();
  const [first, second] = clips;
  if (first) {
    const reach = newRecipe('Reach for the lantern', baseOf(first));
    reach.blanks = [
      { id: 'lantern', label: 'The prop the hand reaches', kind: 'prop', prop, point: null },
      {
        id: 'shelf',
        label: 'Where the eyes go after',
        kind: 'point',
        prop: null,
        point: [0.35, 1.45, 0.6],
      },
    ];
    reach.layers = [
      layer('l-1', 'reach', 'rightHand', { start: 0.1, end: 0.35, target: 'lantern' }),
      layer('l-2', 'look-at', 'head', { start: 0.1, end: 0.9, weight: 0.8, target: 'shelf' }),
      layer('l-3', 'wave', 'leftLowerArm', { start: 0.65, end: 0.95, cycles: 2, amount: 25 }),
    ];
    // Saved to the recipe library as r1, which has r3 since (see recipe-library-fixtures.ts).
    reach.from = { entry: 'reach-for-the-lantern', uid: REACH_UID, revision: 1 };
    out.set('reach-for-the-lantern', reach);
  }
  if (second) {
    const look = newRecipe('Look around', { ...baseOf(second), hash: 'old' });
    look.blanks = [
      { id: 'torch', label: 'The torch on the wall', kind: 'prop', prop: 'old-torch', point: null },
    ];
    look.layers = [layer('l-1', 'look-at', 'head', { start: 0.2, end: 0.8, target: 'torch' })];
    out.set('look-around', look);
  }
  const joined = joinedRecipe(clips);
  if (joined) out.set('slash-into-block', joined);
  return out;
}

/**
 * Frostveil's demo of joined clips: a slash trimmed and sped up in the
 * middle, blended into a block that freezes on its guard (the library has
 * the block with other keys), then an idle, with a nudge laid over.
 */
function joinedRecipe(clips: readonly AnimationClip[]): RecipeFile | null {
  const named = (name: string) => clips.find(c => c.name === name);
  const [slash, block, idle] = [
    named('Sword And Shield Slash'),
    named('Sword And Shield Block'),
    named('Sword And Shield Idle'),
  ];
  if (!slash || !block || !idle) return null;
  const recipe = newRecipe('Slash into block', { ...baseOf(slash), trim: [4, 44] });
  recipe.version = 2;
  recipe.joined = [
    {
      ...baseOf(block),
      hash: 'old',
      trim: [2, 28],
      blend: { frames: 6, curve: 'smooth', lineUp: 'hips-and-facing' },
    },
    { ...baseOf(idle), trim: [0, 40], blend: { frames: 8, curve: 'smooth', lineUp: 'hips' } },
  ];
  recipe.time = [
    { kind: 'retime', clip: 0, from: 14, to: 26, speed: 1.6, ease: 2 },
    { kind: 'freeze', clip: 1, at: 20, frames: 12, breathe: 0.4, easeIn: 3, easeOut: 3 },
  ];
  recipe.layers = [
    layer('l-1', 'nudge', 'spine', { start: 0.3, end: 0.7, rotation: [8, 0, 0], weight: 0.8 }),
  ];
  return recipe;
}

export function createRecipeHandlers(ctx: RecipeContext) {
  const byProject = new Map<string, Map<string, RecipeFile>>();
  let seeded = ctx.state !== 'demo';

  const recipes = (project: unknown) => {
    if (!seeded) {
      seeded = true;
      const prop =
        (ctx.project('frostveil').assets ?? []).find(a => a.kind !== 'character')?.id ?? null;
      byProject.set('frostveil', demoRecipes(ctx.clips(), prop));
    }
    const id = String(project);
    let found = byProject.get(id);
    if (!found) {
      found = new Map();
      byProject.set(id, found);
    }
    return found;
  };
  const existing = (project: unknown, id: unknown) => {
    const recipe = recipes(project).get(String(id));
    if (!recipe) fail('not-found', `Not found: the recipe ${String(id)}`);
    return recipe;
  };
  const checkName = (name: string) => {
    if (!name.trim()) fail('invalid', 'Give the recipe a name.');
    if (name.trim().length > 120) fail('invalid', 'Keep the recipe under 120 characters.');
  };
  const hasAsset = (project: string, id: string) =>
    (ctx.project(project).assets ?? []).some(a => a.id === id);
  /** The recipe as read for `project`: gone props open, a warning and an entry each. */
  const settled = (project: string, recipe: RecipeFile): [RecipeFile, string[], GoneProp[]] => {
    const warnings: string[] = [];
    const gone: GoneProp[] = [];
    const blanks = recipe.blanks.map(b => {
      if (b.kind !== 'prop' || !b.prop || hasAsset(project, b.prop)) return b;
      warnings.push(
        `The blank ${b.id} names ${b.prop}, which is no longer an asset of ${ctx.project(project).name}, so it reads as open. The file keeps ${b.prop}: restoring the asset fills it again, and clearing the blank empties it for good.`
      );
      gone.push({ blank: b.id, prop: b.prop });
      return { ...b, prop: null };
    });
    return [{ ...recipe, blanks }, warnings, gone];
  };
  const doc = (project: string, id: string, stored: RecipeFile): RecipeDoc => {
    const [recipe, warnings, gone] = settled(project, stored);
    return {
      id,
      file: fileOf(id),
      recipe,
      base: matchBase(ctx.clips(), recipe.base)[0],
      ...(recipe.joined?.length
        ? { joined: recipe.joined.map(c => matchBase(ctx.clips(), c)[0]) }
        : {}),
      mixamo: recipeMixamo(ctx.clips(), recipe),
      warnings,
      gone,
      version: versionOf(stored),
      motionVersion: motionVersionOf(stored),
    };
  };
  const summary = (project: string, id: string, stored: RecipeFile): RecipeSummary => {
    const [recipe, warnings] = settled(project, stored);
    return {
      id,
      file: fileOf(id),
      name: recipe.name,
      base: recipe.base.name,
      frames: recipe.base.frames,
      mixamo: recipe.base.mixamo || (recipe.joined ?? []).some(c => c.mixamo),
      layers: recipe.layers.length,
      blanks: recipe.blanks.length,
      open: recipe.blanks.filter(b => (b.kind === 'prop' ? !b.prop : !b.point)).length,
      edited: recipe.edited,
      version: versionOf(stored),
      motionVersion: motionVersionOf(stored),
      warnings,
      problem: null,
      from: stored.from ?? null,
      format: stored.version,
    };
  };
  /**
   * Sent blanks checked against the project, a sent open blank keeping a
   * gone prop unless `clear` names it.
   */
  const blanksFor = (
    project: string,
    current: RecipeFile,
    sent: RecipeBlank[],
    clear: readonly string[]
  ) =>
    sent.map(b => {
      if (b.kind !== 'prop') return b;
      if (b.prop && !hasAsset(project, b.prop))
        fail(
          'invalid',
          `The blank ${b.id} names ${b.prop}, which is not an asset of ${ctx.project(project).name}.`
        );
      const stored = current.blanks.find(c => c.id === b.id && c.kind === 'prop')?.prop;
      const keep = !b.prop && stored && !hasAsset(project, stored) && !clear.includes(b.id);
      return keep ? { ...b, prop: stored } : b;
    });
  const freeId = (project: unknown, name: string) => {
    // A name with no letters or digits makes 'recipe', as the core makes it.
    const base = /[a-z0-9]/i.test(name) ? slugOf(name) : 'recipe';
    let id = base;
    for (let n = 2; recipes(project).has(id); n += 1) id = `${base}-${n}`;
    return id;
  };

  return {
    ...createRecipeLibraryHandlers({
      clips: ctx.clips,
      project: ctx.project,
      recipes: id => recipes(id),
      doc,
      freeId: (project, name) => freeId(project, name),
      state: ctx.state,
    }),
    ...createRecipeBakeHandlers({
      doc: (project, id) => doc(project, id, existing(project, id)),
      asset: (project, id) => {
        const found = (ctx.project(project).assets ?? []).find(a => a.id === id);
        if (!found) return null;
        const version = Math.max(1, ...(found.versions ?? []).map(v => v.version));
        return { name: found.name, version };
      },
      addVersion: (project, id) => {
        const p = ctx.project(project);
        const found = (p.assets ?? []).find(a => a.id === id);
        if (!found) fail('not-found', `Not found: the asset ${id}`);
        const last = found.versions[found.versions.length - 1];
        const number = (last?.version ?? 0) + 1;
        const file = `models/${id.replace(/-/g, '_')}_v${number}.glb`;
        const at = clock.now().toISOString();
        const version = { version: number, file, createdAt: at, stats: last?.stats ?? null };
        ctx.saveProject({
          ...p,
          assets: (p.assets ?? []).map(a =>
            a.id === id ? { ...a, versions: [...a.versions, version], updatedAt: at } : a
          ),
        });
        const dir = `${ctx.root()}/projects/${project}`;
        if (last) ctx.alias(`${dir}/${file}`, `${dir}/${last.file}`);
        return number;
      },
      stage: (project, id, file) => {
        const found = (ctx.project(project).assets ?? []).find(a => a.id === id);
        const last = found?.versions[found.versions.length - 1];
        if (last) ctx.alias(file, `${ctx.root()}/projects/${project}/${last.file}`);
      },
      demoModel: () => {
        const assets = ctx.project('frostveil').assets ?? [];
        return (
          (assets.find(a => a.id === 'clay-figure') ?? assets.find(a => a.id === 'elsheria'))?.id ??
          'elsheria'
        );
      },
      standInClip: asset => STAND_IN_CLIPS[asset] ?? null,
      state: ctx.state,
    }),
    recipe_list: ({ project }: Args) =>
      [...recipes(project)]
        .map(([id, recipe]) => summary(String(project), id, recipe))
        .sort((a, b) => a.id.localeCompare(b.id)),
    recipe_get: ({ project, id }: Args) => doc(String(project), String(id), existing(project, id)),
    recipe_create: ({ project, clip, name }: Args) => {
      // No clip: the recipe runs on the model's rest pose.
      const found = clip == null ? null : ctx.clips().find(c => c.id === String(clip));
      if (clip != null && !found) fail('not-found', `Not found: the library clip ${String(clip)}`);
      const base = found ? baseOf(found) : restBase();
      const named = typeof name === 'string' && name.trim() ? name.trim() : base.name;
      checkName(named);
      const id = freeId(project, named);
      const recipe = newRecipe(named, base);
      recipes(project).set(id, recipe);
      return doc(String(project), id, recipe);
    },
    recipe_rename: ({ project, id, name }: Args) => {
      checkName(String(name));
      const recipe = { ...existing(project, id), name: String(name).trim() };
      checkSize(recipe);
      recipes(project).set(String(id), recipe);
      return summary(String(project), String(id), recipe);
    },
    recipe_update: ({ project, id, change }: Args) => {
      const {
        name,
        layers,
        blanks,
        clips,
        kept: keptFrom,
        time,
        mirror,
        expected,
        clear = [],
      } = change as RecipeChange;
      if (name != null) checkName(name);
      const current = existing(project, id);
      const now = versionOf(current);
      if (expected != null && expected !== now)
        fail(
          'moved',
          `The recipe ${String(id)} changed since it was read (its version is now ${now}). Read it again and send the change once more.`
        );
      const kept = blanks
        ? blanksFor(String(project), current, blanks, clear)
        : current.blanks.map(b => (clear.includes(b.id) ? { ...b, prop: null } : b));
      for (const c of clear)
        if (!kept.some(b => b.id === c && b.kind === 'prop'))
          fail('invalid', `The recipe has no prop blank ${c} to clear.`);
      const [head, ...joined] = clips ?? [current.base, ...(current.joined ?? [])];
      if (!head) fail('invalid', 'Send at least one clip: the first is the base.');
      const followed = clips
        ? followClips(current, clips, keptFrom, layers != null, time != null)
        : { layers: current.layers, time: current.time ?? [] };
      const next: RecipeFile = withVersion({
        ...current,
        name: name?.trim() ?? current.name,
        base: head,
        joined,
        mirror: mirror ?? current.mirror,
        time: time ?? followed.time,
        layers: (layers ?? followed.layers).map((l, i) => (l.id ? l : { ...l, id: `l-${i + 1}` })),
        blanks: kept,
      });
      const problems = checkDraft(next);
      const first = [
        ...problems.general,
        ...Object.values(problems.blanks),
        ...Object.values(problems.layers),
        v2Problem(next),
      ].filter(Boolean)[0];
      if (first) fail('invalid', first);
      const recipe = { ...next, edited: clock.now().toISOString() };
      checkSize(recipe);
      recipes(project).set(String(id), recipe);
      return doc(String(project), String(id), recipe);
    },
    recipe_remove: ({ project, id }: Args) => {
      existing(project, id);
      recipes(project).delete(String(id));
      return `/trash/${String(project)}/recipes/${String(id)}.recipe.json`;
    },
  } as Record<string, (args: Args) => unknown>;
}
