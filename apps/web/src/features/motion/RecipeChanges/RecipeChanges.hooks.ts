import { useMemo, useState } from 'react';
import type { LayerKind } from '@/ipc/client';
import { LIMITS, checkDraft } from '@/state/recipe-check';
import { ADD_KINDS, KIND_ABOUT, KIND_WORDS, newLayer } from '@/state/recipe-draft';
import { draftedLayer, keysDrafted } from '@/state/recipe-drafts';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

/** The open recipe's changes in order, the one whose settings show, and adding one. */
export function useRecipeChanges() {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const drafts = useRecipeEditorStore(s => s.editing?.doc.drafts ?? null);
  const dt = useRecipeTimeline();
  const picked = useRecipeEditorStore(s => (s.picked?.what === 'layer' ? s.picked.id : null));
  const { edit, pick } = useRecipeEditorStore.getState();
  const [kind, setKind] = useState<LayerKind>('reach');
  const problems = useMemo(() => (draft ? checkDraft(draft).layers : {}), [draft]);
  const layers = draft?.layers ?? [];
  const full = layers.length >= LIMITS.layers;
  return {
    layers,
    /** A change's place on the bake's frames, through the base edits. */
    toFrame: dt?.toFrame ?? (() => 0),
    problems,
    /** What an agent drafted of the change `id`: "2 of 6 keys drafted", "drafted", or null. */
    drafted: (layer: (typeof layers)[number]) =>
      keysDrafted(drafts, layer) ?? (draftedLayer(drafts, layer.id) ? 'drafted' : null),
    picked,
    toggle: (id: string) => pick(picked === id ? null : { what: 'layer', id }),
    // Short labels, as the change form's kind picker has them, so the closed
    // select is never cut off; what the picked kind does shows on hover.
    kinds: ADD_KINDS.map(k => ({ value: k, label: KIND_WORDS[k] })),
    kindAbout: KIND_ABOUT[kind],
    kind,
    setKind,
    addBlocked: full ? `A recipe holds at most ${LIMITS.layers} changes` : undefined,
    add: () => {
      if (!draft || full) return;
      // A grip comes from the grip library: Add a grip's picker shows instead.
      if (kind === 'grip') return pick({ what: 'grips', id: '' });
      const layer = newLayer(kind, draft.layers, draft.blanks);
      edit(d => ({ ...d, layers: [...d.layers, layer] }));
      pick({ what: 'layer', id: layer.id });
    },
  };
}
