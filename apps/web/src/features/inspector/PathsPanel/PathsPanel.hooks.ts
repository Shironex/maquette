import type { ScenePath } from '@/ipc/client';
import { removePath, type PathsDraft } from '@/state/paths';
import { usePathsStore } from '@/state/use-paths-store';

/** The panel's store: the draft, the selection, and edits to the selected path. */
export function usePathsPanel() {
  const draft = usePathsStore(s => s.draft);
  const selection = usePathsStore(s => s.selection);
  const select = usePathsStore(s => s.select);
  const edit = usePathsStore(s => s.edit);
  const path: ScenePath | undefined = selection
    ? draft.paths.find(p => p.id === selection.path)
    : undefined;
  return {
    draft,
    path,
    pick: (id: string) => select({ path: id, point: null }),
    /** Changes the selected path. */
    change: (fn: (draft: PathsDraft, id: string) => PathsDraft) => {
      if (path) edit(d => fn(d, path.id));
    },
    remove: () => {
      if (!path) return;
      const rest = draft.paths.filter(p => p.id !== path.id);
      edit(d => removePath(d, path.id), rest[0] ? { path: rest[0].id, point: null } : null);
    },
  };
}
