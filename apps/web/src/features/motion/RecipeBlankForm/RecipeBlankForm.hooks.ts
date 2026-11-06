import type { BlankKind, RecipeBlank } from '@/ipc/client';
import { checkDraft } from '@/state/recipe-check';
import { withBlankKind, withoutBlank } from '@/state/recipe-draft';
import { useRecipeAssets } from '@/state/use-recipe-assets';
import { clearsOf, useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useBakeModel } from '../RecipeBake';

type Triple = [number, number, number];

/**
 * One blank's name, kind and fill, edited into the open recipe's draft, with
 * the project's assets to pick from. The model the recipe animates is no
 * prop of its own: it is left out, and a blank still filled with it is
 * marked by `checkDraft` (the bake refuses it too).
 */
export function useRecipeBlankForm(id: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const assets = useRecipeAssets();
  const gone = useRecipeEditorStore(s => s.editing?.doc.gone.find(g => g.blank === id) ?? null);
  const cleared = useRecipeEditorStore(s => (s.editing ? clearsOf(s.editing).includes(id) : false));
  const { edit, pick, clearGone } = useRecipeEditorStore.getState();
  const blank = draft?.blanks.find(b => b.id === id) ?? null;
  const model = useBakeModel();
  const problem = draft && blank ? (checkDraft(draft, model).blanks[id] ?? null) : null;
  const choices = assets.filter(a => a.id !== model?.asset || a.id === blank?.prop);
  const put = (next: (blank: RecipeBlank) => RecipeBlank) =>
    edit(d => ({ ...d, blanks: d.blanks.map(b => (b.id === id ? next(b) : b)) }));
  const aiming = draft?.layers.filter(l => l.settings.target === id).length ?? 0;
  return {
    blank,
    problem,
    aiming,
    /** The asset the file still names while this blank reads open, unless it is being emptied. */
    gone: gone && blank?.kind === 'prop' && !blank.prop ? gone.prop : null,
    cleared,
    keepGone: () => clearGone(id, false),
    props: [
      { value: '', label: choices.length ? 'Pick a prop' : 'The project has no props yet' },
      ...choices.map(a => ({ value: a.id, label: a.name })),
    ],
    setLabel: (label: string) => put(b => ({ ...b, label })),
    setKind: (kind: BlankKind) => {
      put(b => withBlankKind(b, kind));
      if (kind !== 'prop') clearGone(id, false);
    },
    setProp: (prop: string) => put(b => ({ ...b, prop: prop || null })),
    setPoint: (axis: number, metres: number) =>
      put(b => {
        const p = [...(b.point ?? [0, 0, 0])] as Triple;
        p[axis] = metres;
        return { ...b, point: p };
      }),
    clear: () => {
      put(b => ({ ...b, prop: null, point: null }));
      // A blank that only reads open keeps its gone prop unless it is cleared for good.
      if (gone) clearGone(id, true);
    },
    remove: () => {
      clearGone(id, false);
      edit(d => withoutBlank(d, id));
      pick(null);
    },
  };
}
