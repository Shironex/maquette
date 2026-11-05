import type { BlankKind, RecipeBlank } from '@/ipc/client';
import { checkDraft } from '@/state/recipe-check';
import { withBlankKind, withoutBlank } from '@/state/recipe-draft';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

type Triple = [number, number, number];

/** One blank's name, kind and fill, edited into the open recipe's draft, with the project's assets to pick from. */
export function useRecipeBlankForm(id: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const assets = useAppStore(s => s.workspace?.project?.assets);
  const { edit, pick } = useRecipeEditorStore.getState();
  const blank = draft?.blanks.find(b => b.id === id) ?? null;
  const problem = draft && blank ? (checkDraft(draft).blanks[id] ?? null) : null;
  const put = (next: (blank: RecipeBlank) => RecipeBlank) =>
    edit(d => ({ ...d, blanks: d.blanks.map(b => (b.id === id ? next(b) : b)) }));
  const aiming = draft?.layers.filter(l => l.settings.target === id).length ?? 0;
  return {
    blank,
    problem,
    aiming,
    props: [
      { value: '', label: assets?.length ? 'Pick a prop' : 'The project has no assets yet' },
      ...(assets ?? []).map(a => ({ value: a.id, label: a.name })),
    ],
    setLabel: (label: string) => put(b => ({ ...b, label })),
    setKind: (kind: BlankKind) => put(b => withBlankKind(b, kind)),
    setProp: (prop: string) => put(b => ({ ...b, prop: prop || null })),
    setPoint: (axis: number, metres: number) =>
      put(b => {
        const p = [...(b.point ?? [0, 0, 0])] as Triple;
        p[axis] = metres;
        return { ...b, point: p };
      }),
    clear: () => put(b => ({ ...b, prop: null, point: null })),
    remove: () => {
      edit(d => withoutBlank(d, id));
      pick(null);
    },
  };
}
