import { useLayoutDirty, useLayoutStore } from '@/state/use-layout-store';

/** The layout, whether it has unsaved edits, and saving or reverting them. */
export function useLayoutBarActions() {
  const view = useLayoutStore(s => s.view);
  const saving = useLayoutStore(s => s.saving);
  const save = useLayoutStore(s => s.save);
  const revert = useLayoutStore(s => s.revert);
  const dirty = useLayoutDirty();
  return { view, saving, save, revert, dirty };
}
