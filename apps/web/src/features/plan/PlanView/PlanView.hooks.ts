import { useEffect, useRef } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * The open project's canvases, read when the project changes; the open
 * canvas saves when the view goes away. An open canvas shows even while
 * the project changes, so a failed save on the way out can still be
 * answered on it.
 */
export function usePlanView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const projectId = project?.id ?? null;
  const loadList = usePlansStore(s => s.loadList);
  const list = usePlansStore(s => s.list);
  const listError = usePlansStore(s => s.listError);
  const open = usePlansStore(s => s.open);
  const storeProject = usePlansStore(s => s.project);
  const opening = usePlansStore(s => s.opening);
  const openError = usePlansStore(s => s.openError);
  const stale = Boolean(projectId) && storeProject !== projectId && !open;
  // Reads the workspace project's canvases when it changes, and again when
  // the store still holds another project's with nothing open (after a
  // switch whose save failed, Stay, then Back), so the list never waits on
  // a load nobody started.
  const loaded = useRef<string | null>(null);
  useEffect(() => {
    if (!projectId || (!stale && loaded.current === projectId)) return;
    loaded.current = projectId;
    void loadList(projectId);
  }, [projectId, stale, loadList]);
  useEffect(() => () => void usePlansStore.getState().save(), []);
  const current = storeProject === projectId;
  return {
    project,
    list: current ? list : null,
    listError: current ? listError : null,
    open,
    opening,
    openError,
    retry: () => (projectId ? void loadList(projectId) : undefined),
  };
}
