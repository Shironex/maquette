/**
 * Remove recipe and its Undo in mock mode: the recipe and its draft marks
 * move to a trash kept in memory, refused while its bake runs, as the core
 * refuses it. Restore puts both back unless a recipe took the id again.
 */
import type { RecipeDrafts, RecipeFile, RemovedRecipe } from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';

interface TrashContext {
  recipes: (project: unknown) => Map<string, RecipeFile>;
  /** Draft marks by `project/id`. */
  marks: Map<string, RecipeDrafts>;
  existing: (project: unknown, id: unknown) => RecipeFile;
  /** Whether the recipe bakes now. */
  baking: (project: string, id: string) => boolean;
}

interface Parked {
  id: string;
  recipe: RecipeFile;
  drafts: RecipeDrafts | undefined;
}

export function createRecipeTrashHandlers(ctx: TrashContext) {
  /** Removed recipes by `project/trash`. */
  const parked = new Map<string, Parked>();
  return {
    recipe_remove: ({ project, id }: Args): RemovedRecipe => {
      const recipe = ctx.existing(project, id);
      const p = String(project);
      const key = `${p}/${String(id)}`;
      if (ctx.baking(p, String(id)))
        fail('invalid', 'This recipe is baking. Wait for the bake or cancel it first.');
      let trash = `${String(id)}-${clock.now().toISOString().replace(/\D/g, '')}`;
      while (parked.has(`${p}/${trash}`)) trash += '2';
      parked.set(`${p}/${trash}`, { id: String(id), recipe, drafts: ctx.marks.get(key) });
      ctx.recipes(project).delete(String(id));
      ctx.marks.delete(key);
      return {
        trash,
        id: String(id),
        name: recipe.name,
        path: `recipes/${trash}.recipe.json`,
      };
    },
    recipe_restore: ({ project, trash }: Args): string => {
      const p = String(project);
      const found = parked.get(`${p}/${String(trash)}`);
      if (!found) fail('not-found', `Not found: the removed recipe ${String(trash)}`);
      if (ctx.recipes(project).has(found.id))
        fail(
          'invalid',
          `A recipe with the id ${found.id} is in the project again, so the removed one stays in the trash. Remove or rename that recipe first.`
        );
      ctx.recipes(project).set(found.id, found.recipe);
      if (found.drafts) ctx.marks.set(`${p}/${found.id}`, found.drafts);
      parked.delete(`${p}/${String(trash)}`);
      return found.id;
    },
  };
}
