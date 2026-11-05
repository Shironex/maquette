import { useMemo } from 'react';
import type { BlankKind } from '@/ipc/client';
import { LIMITS, checkDraft } from '@/state/recipe-check';
import { blankFill, newBlank } from '@/state/recipe-draft';
import { useRecipeAssets } from '@/state/use-recipe-assets';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/** The open recipe's blanks, what fills each, the gone props' warnings and adding one. */
export function useRecipeBlanks() {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const warnings = useRecipeEditorStore(s => s.editing?.doc.warnings ?? []);
  const picked = useRecipeEditorStore(s => (s.picked?.what === 'blank' ? s.picked.id : null));
  const assets = useRecipeAssets();
  const { edit, pick } = useRecipeEditorStore.getState();
  const problems = useMemo(() => (draft ? checkDraft(draft).blanks : {}), [draft]);
  const blanks = draft?.blanks ?? [];
  const full = blanks.length >= LIMITS.blanks;
  const nameOf = (id: string) => assets.find(a => a.id === id)?.name ?? null;
  return {
    blanks: blanks.map(b => ({ blank: b, fill: blankFill(b, nameOf) })),
    warnings,
    problems,
    picked,
    toggle: (id: string) => pick(picked === id ? null : { what: 'blank', id }),
    addBlocked: full ? `A recipe holds at most ${LIMITS.blanks} blanks` : undefined,
    add: (kind: BlankKind) => {
      if (!draft || full) return;
      const blank = newBlank(kind, draft.blanks);
      edit(d => ({ ...d, blanks: [...d.blanks, blank] }));
      pick({ what: 'blank', id: blank.id });
    },
  };
}
