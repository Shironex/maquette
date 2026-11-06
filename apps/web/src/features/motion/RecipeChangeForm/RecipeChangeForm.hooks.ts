import type { LayerKind, LayerSettings, RecipeLayer } from '@/ipc/client';
import { boneWords, bonesFor } from '@/state/recipe-bones';
import { checkDraft } from '@/state/recipe-check';
import { EDITABLE_KINDS, KIND_WORDS, withKind } from '@/state/recipe-draft';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

type Triple = [number, number, number];

/** One change's settings, edited into the open recipe's draft. */
export function useRecipeChangeForm(id: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const frames = useRecipeEditorStore(s => s.editing?.doc.recipe.base.frames ?? 0);
  const { edit, pick } = useRecipeEditorStore.getState();
  const layer = draft?.layers.find(l => l.id === id) ?? null;
  const problem = draft && layer ? (checkDraft(draft).layers[id] ?? null) : null;
  const put = (next: (layer: RecipeLayer) => RecipeLayer) =>
    edit(d => ({ ...d, layers: d.layers.map(l => (l.id === id ? next(l) : l)) }));
  const settle = (patch: Partial<LayerSettings>) =>
    put(l => ({ ...l, settings: { ...l.settings, ...patch } }));
  const blanks = draft?.blanks ?? [];
  return {
    layer,
    frames,
    problem,
    kinds: EDITABLE_KINDS.map(k => ({ value: k, label: KIND_WORDS[k] })),
    bones: layer ? bonesFor(layer.kind).map(b => ({ value: b, label: boneWords(b) })) : [],
    targets: [
      { value: '', label: blanks.length ? 'Pick a blank' : 'No blanks yet' },
      ...blanks.map(b => ({ value: b.id, label: `${b.id}: ${b.label}` })),
    ],
    setKind: (kind: LayerKind) => put(l => withKind(l, kind, blanks)),
    setBone: (bone: string) => put(l => ({ ...l, bone })),
    setTarget: (target: string) => settle({ target: target || null }),
    setWeight: (weight: number) => settle({ weight }),
    setRange: (start: number, end: number) => settle({ start, end }),
    setRotation: (axis: number, deg: number) =>
      put(l => {
        const r = [...(l.settings.rotation ?? [0, 0, 0])] as Triple;
        r[axis] = deg;
        return { ...l, settings: { ...l.settings, rotation: r } };
      }),
    setOffset: (axis: number, metres: number) =>
      put(l => {
        const o = [...(l.settings.offset ?? [0, 0, 0])] as Triple;
        o[axis] = metres;
        const none = o.every(v => v === 0);
        return { ...l, settings: { ...l.settings, offset: none ? null : o } };
      }),
    setCycles: (cycles: number) => settle({ cycles }),
    setAmount: (amount: number) => settle({ amount }),
    remove: () => {
      edit(d => ({ ...d, layers: d.layers.filter(l => l.id !== id) }));
      pick(null);
    },
  };
}
