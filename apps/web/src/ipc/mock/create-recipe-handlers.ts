/**
 * Animation recipes in mock mode: each project's recipes kept in memory,
 * made from the mock clip library, with the core's checks that matter to
 * the view (a name, a free id, a known clip, a recipe that still exists).
 * Base clips match by name and frame count as the core matches them. With
 * `?library=demo`, Frostveil opens with two recipes, one whose base clip
 * the library has with other keys.
 */
import type {
  AnimationClip,
  BaseMatch,
  RecipeBase,
  RecipeChange,
  RecipeDoc,
  RecipeFile,
  RecipeSummary,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { slugOf } from './plan-store';

interface RecipeContext {
  clips: () => readonly AnimationClip[];
  state: string | null;
}

const fileOf = (id: string) => `recipes/${id}.recipe.json`;

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

/** Frostveil's demo recipes: one on a library clip, one whose clip came back with new keys. */
function demoRecipes(clips: readonly AnimationClip[]): Map<string, RecipeFile> {
  const out = new Map<string, RecipeFile>();
  const [first, second] = clips;
  if (first) {
    const reach = newRecipe('Reach for the lantern', baseOf(first));
    reach.blanks = [
      { id: 'lantern', label: 'The prop the hand reaches', kind: 'prop', prop: null, point: null },
    ];
    reach.layers = [
      {
        id: 'l-1',
        kind: 'reach',
        bone: 'rightHand',
        settings: {
          weight: 1,
          start: 0.2,
          end: 0.6,
          target: 'lantern',
          rotation: null,
          offset: null,
          cycles: null,
          amount: null,
        },
      },
    ];
    out.set('reach-for-the-lantern', reach);
  }
  if (second) out.set('look-around', newRecipe('Look around', { ...baseOf(second), hash: 'old' }));
  return out;
}

export function createRecipeHandlers(ctx: RecipeContext) {
  const byProject = new Map<string, Map<string, RecipeFile>>();
  let seeded = ctx.state !== 'demo';

  const recipes = (project: unknown) => {
    if (!seeded) {
      seeded = true;
      byProject.set('frostveil', demoRecipes(ctx.clips()));
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
  const doc = (id: string, recipe: RecipeFile): RecipeDoc => {
    const [base, clip] = matchBase(ctx.clips(), recipe.base);
    return {
      id,
      file: fileOf(id),
      recipe,
      base,
      mixamo: recipe.base.mixamo || clip?.rig.kind === 'mixamo',
    };
  };
  const summary = (id: string, recipe: RecipeFile): RecipeSummary => ({
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
    problem: null,
  });
  const freeId = (project: unknown, name: string) => {
    // A name with no letters or digits makes 'recipe', as the core makes it.
    const base = /[a-z0-9]/i.test(name) ? slugOf(name) : 'recipe';
    let id = base;
    for (let n = 2; recipes(project).has(id); n += 1) id = `${base}-${n}`;
    return id;
  };

  return {
    recipe_list: ({ project }: Args) =>
      [...recipes(project)]
        .map(([id, recipe]) => summary(id, recipe))
        .sort((a, b) => a.id.localeCompare(b.id)),
    recipe_get: ({ project, id }: Args) => doc(String(id), existing(project, id)),
    recipe_create: ({ project, clip, name }: Args) => {
      const found = ctx.clips().find(c => c.id === String(clip));
      if (!found) fail('not_found', `Not found: the library clip ${String(clip)}`);
      const named = typeof name === 'string' && name.trim() ? name.trim() : found.name;
      checkName(named);
      const id = freeId(project, named);
      const recipe = newRecipe(named, baseOf(found));
      recipes(project).set(id, recipe);
      return doc(id, recipe);
    },
    recipe_rename: ({ project, id, name }: Args) => {
      checkName(String(name));
      const recipe = { ...existing(project, id), name: String(name).trim() };
      recipes(project).set(String(id), recipe);
      return summary(String(id), recipe);
    },
    recipe_update: ({ project, id, change }: Args) => {
      const { layers, blanks } = change as RecipeChange;
      const current = existing(project, id);
      const recipe: RecipeFile = {
        ...current,
        layers: layers ?? current.layers,
        blanks: blanks ?? current.blanks,
        edited: clock.now().toISOString(),
      };
      recipes(project).set(String(id), recipe);
      return doc(String(id), recipe);
    },
    recipe_remove: ({ project, id }: Args) => {
      existing(project, id);
      recipes(project).delete(String(id));
      return `/trash/${String(project)}/recipes/${String(id)}.recipe.json`;
    },
  } as Record<string, (args: Args) => unknown>;
}
