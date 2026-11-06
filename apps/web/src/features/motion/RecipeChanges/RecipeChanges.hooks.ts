import { useMemo, useState } from 'react';
import type { LayerKind } from '@/ipc/client';
import { LIMITS, checkDraft } from '@/state/recipe-check';
import { EDITABLE_KINDS, KIND_ABOUT, KIND_WORDS, newLayer } from '@/state/recipe-draft';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/** The open recipe's changes in order, the one whose settings show, and adding one. */
export function useRecipeChanges() {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const frames = useRecipeEditorStore(s => s.editing?.doc.recipe.base.frames ?? 0);
  const picked = useRecipeEditorStore(s => (s.picked?.what === 'layer' ? s.picked.id : null));
  const { edit, pick } = useRecipeEditorStore.getState();
  const [kind, setKind] = useState<LayerKind>('reach');
  const problems = useMemo(() => (draft ? checkDraft(draft).layers : {}), [draft]);
  const layers = draft?.layers ?? [];
  const full = layers.length >= LIMITS.layers;
  return {
    layers,
    frames,
    problems,
    picked,
    toggle: (id: string) => pick(picked === id ? null : { what: 'layer', id }),
    // Short labels, as the change form's kind picker has them, so the closed
    // select is never cut off; what the picked kind does shows on hover.
    kinds: EDITABLE_KINDS.map(k => ({ value: k, label: KIND_WORDS[k] })),
    kindAbout: KIND_ABOUT[kind],
    kind,
    setKind,
    addBlocked: full ? `A recipe holds at most ${LIMITS.layers} changes` : undefined,
    add: () => {
      if (!draft || full) return;
      const layer = newLayer(kind, draft.layers, draft.blanks);
      edit(d => ({ ...d, layers: [...d.layers, layer] }));
      pick({ what: 'layer', id: layer.id });
    },
  };
}
