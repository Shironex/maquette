import { useEffect, useState } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useTrashStore } from '@/state/use-trash-store';

/**
 * The open project's trash and its Restore. It loads again when the
 * project's assets come or go, so a remove by an agent shows up too.
 */
export function useRecentlyRemoved() {
  const projectId = useAppStore(s => s.workspace?.project?.id ?? null);
  const assetCount = useAppStore(s => s.workspace?.project?.assets?.length ?? 0);
  const { project, entries, restoring, load, restore } = useTrashStore();
  const [open, setOpen] = useState(false);

  useEffect(() => setOpen(false), [projectId]);
  useEffect(() => {
    if (projectId) void load(projectId);
  }, [projectId, assetCount, load]);

  const shown = projectId && project === projectId ? entries : [];
  const restoreOne = (id: string) => {
    if (projectId) void restore(projectId, [id]);
  };
  return { entries: shown, restoring, open, setOpen, restoreOne };
}
