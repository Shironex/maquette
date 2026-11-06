import { useMemo, useState } from 'react';
import type { RecipeOutside } from '@/state/recipe-editor.types';
import { checkDraft, problemCount } from '@/state/recipe-check';
import { blankFilled } from '@/state/recipe-draft';
import { isDirty, useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';
import { useBakeModel, useRecipeBake } from '../RecipeBake';

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
  const unreadable = useRecipeEditorStore(s => s.unreadable);
  const outside = useRecipeEditorStore(s => s.outside);
  const leaving = useRecipeEditorStore(s => s.leaving);
  // Escape closes the clash question without choosing; saves stay blocked until it is answered.
  const [hidden, setHidden] = useState<RecipeOutside | null>(null);
  const draft = editing?.draft ?? null;
  const model = useBakeModel();
  const problems = useMemo(() => (draft ? checkDraft(draft, model) : null), [draft, model]);
  const dt = useRecipeTimeline();
  const store = useRecipeEditorStore.getState();
  const dirty = isDirty(editing);
  const toFix = problems ? problemCount(problems) : 0;
  const status = saving ? 'saving' : dirty ? 'unsaved edits' : 'saved';
  const clips = draft?.clips.map(c => c.name).join(' + ') ?? '';
  return {
    editing,
    dirty,
    /** "Slash + Block + Idle · 147 f · slash.recipe.json · saved". */
    meta: [clips, `${dt?.timeline.frames ?? 0} f`, editing?.doc.file ?? '', status].join(' · '),
    saving,
    saveError,
    merged,
    gone,
    unreadable,
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

/**
 * The open recipe's Apply and its bake. Unsaved edits are saved first
 * (Save and apply), so the bake is the recipe on show; a save that cannot
 * go through keeps Apply off with its reason.
 */
export function useEditorApply(saveBlocked: string | undefined) {
  const editing = useRecipeEditorStore(s => s.editing);
  const saving = useRecipeEditorStore(s => s.saving);
  const draft = editing?.draft;
  const bake = useRecipeBake(
    editing?.project ?? '',
    editing?.id ?? '',
    {
      open: draft?.blanks.filter(b => !blankFilled(b)).length ?? 0,
      layers: draft?.layers.length ?? 0,
    },
    { version: editing?.doc.version ?? null, motionVersion: editing?.doc.motionVersion ?? null }
  );
  const dirty = isDirty(editing);
  const apply = async () => {
    const store = useRecipeEditorStore.getState();
    if (dirty && !(await store.save())) return;
    await bake.apply(useRecipeEditorStore.getState().editing?.doc.version ?? null);
  };
  return {
    bake,
    dirty,
    // A marked entry blocks Apply with or without unsaved edits: the file
    // may hold it too (a prop blank holding the model it bakes onto).
    blocked: bake.blocked ?? saveBlocked,
    loading: saving || (bake.busy && !bake.running),
    apply: () => void apply(),
  };
}
