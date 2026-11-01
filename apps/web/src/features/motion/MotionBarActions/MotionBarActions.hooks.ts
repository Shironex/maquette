import { useEffect } from 'react';
import { useModelBudget } from '@/features/common/SceneBudgetMeter/SceneBudgetMeter.hooks';
import { useAppStore } from '@/state/use-app-store';
import { budgetLine, budgetPending, budgetTip } from '@/state/motion';
import { useMotionDirty, useMotionStore } from '@/state/use-motion-store';

/** The scene budget with the draft's clips, and saving or reverting the draft on Mod+S. */
export function useMotionBarActions() {
  const hasProject = useAppStore(s => Boolean(s.workspace?.project));
  const view = useMotionStore(s => s.view);
  const loadFailed = useMotionStore(s => s.error !== null);
  const draft = useMotionStore(s => s.draft);
  const saving = useMotionStore(s => s.saving);
  const save = useMotionStore(s => s.save);
  const revert = useMotionStore(s => s.revert);
  const dirty = useMotionDirty();
  const models = useModelBudget();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== 's') return;
      e.preventDefault();
      if (dirty && !models) void save();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dirty, save, models]);
  const line = view ? budgetLine(view, draft) : null;
  return {
    view,
    pending: budgetPending(hasProject, loadFailed),
    line,
    tip: line
      ? budgetTip(line)
      : hasProject
        ? 'The scene budget is read with the motion.'
        : 'Open a project to see its scene budget.',
    saving,
    save,
    revert,
    dirty,
    models,
  };
}
