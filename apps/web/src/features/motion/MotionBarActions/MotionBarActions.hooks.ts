import { useEffect } from 'react';
import { budgetLine, budgetTip } from '@/state/motion';
import { useMotionDirty, useMotionStore } from '@/state/use-motion-store';

/** The scene budget with the draft's clips, and saving or reverting the draft on Mod+S. */
export function useMotionBarActions() {
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  const saving = useMotionStore(s => s.saving);
  const save = useMotionStore(s => s.save);
  const revert = useMotionStore(s => s.revert);
  const dirty = useMotionDirty();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== 's') return;
      e.preventDefault();
      if (dirty) void save();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dirty, save]);
  const line = view ? budgetLine(view, draft) : null;
  return { view, line, tip: line ? budgetTip(line) : '', saving, save, revert, dirty };
}
