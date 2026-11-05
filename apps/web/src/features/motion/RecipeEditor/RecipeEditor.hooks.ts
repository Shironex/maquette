import { useMemo, useState } from 'react';
import type { RecipeOutside } from '@/state/recipe-editor.types';
import { checkDraft, problemCount } from '@/state/recipe-check';
import { isDirty, useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/**
 * The open recipe's header and its questions: unsaved edits, Save and
 * Discard, what came in from outside, and the answers to a stopped leave or
 * an outside clash.
 */
export function useRecipeEditor() {
  const editing = useRecipeEditorStore(s => s.editing);
  const saving = useRecipeEditorStore(s => s.saving);
  const saveError = useRecipeEditorStore(s => s.saveError);
  const merged = useRecipeEditorStore(s => s.merged);
  const gone = useRecipeEditorStore(s => s.gone);
  const outside = useRecipeEditorStore(s => s.outside);
  const leaving = useRecipeEditorStore(s => s.leaving);
  // Escape closes the clash question without choosing; saves stay blocked until it is answered.
  const [hidden, setHidden] = useState<RecipeOutside | null>(null);
  const draft = editing?.draft ?? null;
  const problems = useMemo(() => (draft ? checkDraft(draft) : null), [draft]);
  const store = useRecipeEditorStore.getState();
  const dirty = isDirty(editing);
  const toFix = problems ? problemCount(problems) : 0;
  return {
    editing,
    dirty,
    saving,
    saveError,
    merged,
    gone,
    general: problems?.general ?? [],
    toFix,
    saveBlocked: outside
      ? 'Choose Keep mine or Use theirs first'
      : toFix
        ? `Fix ${toFix === 1 ? 'the marked entry' : `the ${toFix} marked entries`} first`
        : !dirty
          ? 'Nothing to save'
          : undefined,
    save: () => void store.save(),
    discard: store.discard,
    back: () => void store.leave({ to: 'list' }),
    leave: leaving
      ? {
          blocked: toFix
            ? 'Fix the marked entries first, or leave without saving'
            : outside
              ? 'Choose Keep mine or Use theirs first'
              : undefined,
          save: () => void store.saveAndLeave(),
          anyway: () => void store.leaveAnyway(),
          stay: store.stay,
        }
      : null,
    outside: outside
      ? {
          clashes: outside.merge.clashes,
          asking: hidden !== outside,
          ask: () => setHidden(null),
          later: () => setHidden(outside),
          keepMine: store.keepMine,
          useTheirs: store.useTheirs,
        }
      : null,
  };
}
