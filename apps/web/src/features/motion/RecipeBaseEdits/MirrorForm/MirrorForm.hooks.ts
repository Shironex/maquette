import { useState } from 'react';
import { swapSides, swapsOf } from '@/state/recipe-clips';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/**
 * The open recipe's mirror: on or off, and when the changes name a side,
 * whether turning it swaps them too (asked first).
 */
export function useMirrorForm() {
  const editing = useRecipeEditorStore(s => s.editing);
  const [asking, setAsking] = useState<boolean | null>(null);
  const draft = editing?.draft ?? null;
  const { edit } = useRecipeEditorStore.getState();
  const swaps = draft ? swapsOf(draft.layers) : [];
  const turn = (on: boolean, swap: boolean) => {
    edit(d => ({ ...d, mirror: on, layers: swap ? swapSides(d.layers) : d.layers }));
    setAsking(null);
  };
  return {
    on: draft?.mirror ?? false,
    swaps: swaps.map(s => ({ key: s.layer.id, line: `${s.layer.id}: ${s.layer.bone} to ${s.to}` })),
    asking,
    set: (on: boolean) => {
      if (!draft || on === draft.mirror) return;
      if (swaps.length) setAsking(on);
      else turn(on, false);
    },
    turn,
    cancel: () => setAsking(null),
    swapNow: () => edit(d => ({ ...d, layers: swapSides(d.layers) })),
  };
}
