import { useEffect, useRef } from 'react';
import { commands } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { useAppStore } from '@/state/use-app-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * Keeps the Plan view in step with the project's `plans/` folder: the core
 * watches it (an agent's write, another window, a sync from another
 * computer) and says which canvas changed. The open canvas reads its file
 * and change log again (its own saves read back as the same canvas, so
 * nothing reloads in a loop); with no canvas open the list reads again.
 */
export function usePlanWatch(projectId: string | null) {
  const openId = usePlansStore(s => s.open?.id ?? null);
  useEffect(() => {
    // Without the watch the canvas still works; it just does not see outside edits.
    if (projectId) void commands.planWatch(projectId).catch(() => undefined);
  }, [projectId]);
  useEffect(() => {
    const stop = events.planChanged.listen(e => {
      const { project, id } = e.payload;
      const plans = usePlansStore.getState();
      if (plans.project !== project) return;
      if (plans.open?.id === id) {
        void plans.reload();
        void usePlanLogStore.getState().load(project, id);
      } else if (!plans.open && !plans.opening) void plans.loadList(project);
    });
    // The listener may already be gone with the app (or a test's IPC mock).
    return () => void stop.then(fn => fn()).catch(() => undefined);
  }, []);
  useEffect(() => {
    const project = usePlansStore.getState().project;
    const log = usePlanLogStore.getState();
    if (project && openId) void log.load(project, openId);
    else log.clear();
  }, [openId]);
}

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
  usePlanWatch(projectId);
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
