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
 * torch prop is gone.
 */
import type {
  AnimationClip,
  BaseMatch,
  GoneProp,
  ProjectManifest,
  RecipeBlank,
  RecipeLayer,
  RecipeBase,
  RecipeChange,
  RecipeDoc,
  RecipeFile,
  RecipeSummary,
} from '../client';
import { clock } from '@/lib/clock';
import { checkDraft } from '@/state/recipe-check';
import { fail, type Args } from './backend-options';
import { createRecipeBakeHandlers } from './create-recipe-bake-handlers';
import { slugOf } from './plan-store';

interface RecipeContext {
  clips: () => readonly AnimationClip[];
  project: (id: string) => ProjectManifest;
  state: string | null;
}

const fileOf = (id: string) => `recipes/${id}.recipe.json`;

/** A stand-in for the core's hash of the file: 16 hex digits that change with any content. */
function versionOf(recipe: RecipeFile): string {
  const text = JSON.stringify(recipe);
  let a = 0x811c9dc5;
  let b = 0x01000193;
  for (let i = 0; i < text.length; i += 1) {
    a = Math.imul(a ^ text.charCodeAt(i), 0x01000193);
    b = Math.imul(b ^ text.charCodeAt(i), 0x811c9dc5);
  }
  const hex = (n: number) => (n >>> 0).toString(16).padStart(8, '0');
  return hex(a) + hex(b);
}

const baseOf = (clip: AnimationClip): RecipeBase => ({
  name: clip.name,
  frames: clip.frames,
  length: clip.length ?? 0,
  hash: clip.hash,
  mixamo: clip.rig.kind === 'mixamo',
});

function matchBase(clips: readonly AnimationClip[], base: RecipeBase): [BaseMatch, AnimationClip?] {
  const name = base.name.trim().toLowerCase();
  const same = clips.filter(c => c.frames === base.frames && c.name.trim().toLowerCase() === name);
  const exact = same.find(c => c.hash === base.hash);
  if (exact) return [{ status: 'found', clip: exact.id, warning: null }, exact];
  const other = same[0];
  if (other)
    return [
      {
        status: 'hash-differs',
        clip: other.id,
        warning: `The library's ${other.name} has other keys than the recipe was made on (its hash differs). A bake will use the library's.`,
      },
      other,
    ];
  return [
    {
      status: 'missing',
      clip: null,
      warning: `The animation library has no clip called ${base.name} with ${base.frames} frames. Import it before baking.`,
    },
  ];
}

function newRecipe(name: string, base: RecipeBase): RecipeFile {
  const now = clock.now().toISOString();
  return { version: 1, name, revision: 1, created: now, edited: now, base, blanks: [], layers: [] };
}

const layer = (
  id: string,
  kind: RecipeLayer['kind'],
  bone: string,
  settings: Partial<RecipeLayer['settings']>
): RecipeLayer => ({
  id,
  kind,
  bone,
  settings: {
    weight: 1,
    start: 0,
    end: 1,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
    ...settings,
  },
});

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
  return out;
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
    if (!recipe) fail('not_found', `Not found: the recipe ${String(id)}`);
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
    const [base, clip] = matchBase(ctx.clips(), recipe.base);
    return {
      id,
      file: fileOf(id),
      recipe,
      base,
      mixamo: recipe.base.mixamo || clip?.rig.kind === 'mixamo',
      warnings,
      gone,
      version: versionOf(stored),
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
      mixamo: recipe.base.mixamo,
      layers: recipe.layers.length,
      blanks: recipe.blanks.length,
      open: recipe.blanks.filter(b => (b.kind === 'prop' ? !b.prop : !b.point)).length,
      edited: recipe.edited,
      version: versionOf(stored),
      warnings,
      problem: null,
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
    ...createRecipeBakeHandlers({
      doc: (project, id) => doc(project, id, existing(project, id)),
      asset: (project, id) => {
        const found = (ctx.project(project).assets ?? []).find(a => a.id === id);
        if (!found) return null;
        const version = Math.max(1, ...(found.versions ?? []).map(v => v.version));
        return { name: found.name, version };
      },
      state: ctx.state,
    }),
    recipe_list: ({ project }: Args) =>
      [...recipes(project)]
        .map(([id, recipe]) => summary(String(project), id, recipe))
        .sort((a, b) => a.id.localeCompare(b.id)),
    recipe_get: ({ project, id }: Args) => doc(String(project), String(id), existing(project, id)),
    recipe_create: ({ project, clip, name }: Args) => {
      const found = ctx.clips().find(c => c.id === String(clip));
      if (!found) fail('not_found', `Not found: the library clip ${String(clip)}`);
      const named = typeof name === 'string' && name.trim() ? name.trim() : found.name;
      checkName(named);
      const id = freeId(project, named);
      const recipe = newRecipe(named, baseOf(found));
      recipes(project).set(id, recipe);
      return doc(String(project), id, recipe);
    },
    recipe_rename: ({ project, id, name }: Args) => {
      checkName(String(name));
      const recipe = { ...existing(project, id), name: String(name).trim() };
      recipes(project).set(String(id), recipe);
      return summary(String(project), String(id), recipe);
    },
    recipe_update: ({ project, id, change }: Args) => {
      const { name, layers, blanks, expected, clear = [] } = change as RecipeChange;
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
      const next: RecipeFile = {
        ...current,
        name: name?.trim() ?? current.name,
        layers: (layers ?? current.layers).map((l, i) => (l.id ? l : { ...l, id: `l-${i + 1}` })),
        blanks: kept,
      };
      const problems = checkDraft(next);
      const first = [
        ...problems.general,
        ...Object.values(problems.blanks),
        ...Object.values(problems.layers),
      ][0];
      if (first) fail('invalid', first);
      const recipe = { ...next, edited: clock.now().toISOString() };
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
