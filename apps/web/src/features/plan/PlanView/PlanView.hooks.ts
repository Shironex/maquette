import { useEffect, useRef, useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { useAppStore } from '@/state/use-app-store';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';

/** Reads the open canvas, its log and proposals again when `id` is the one open (null: whichever is), else the list. */
function reread(project: string, id: string | null) {
  const plans = usePlansStore.getState();
  if (plans.open && (id === null || plans.open.id === id)) {
    void plans.reload();
    void usePlanLogStore.getState().load(project, plans.open.id);
    void usePlanExtrasStore.getState().load(project, plans.open.id);
  } else if (!plans.open && !plans.opening) void plans.loadList(project);
}

/**
 * Keeps the Plan view in step with the project's `plans/` folder: the core
 * watches it (an agent's write, another window, a sync from another
 * computer) and says which canvas changed. The open canvas reads its file
 * and change log again (its own saves read back as the same canvas, so
 * nothing reloads in a loop); with no canvas open the list reads again.
 * When the folder cannot be watched the person is told, and the canvas
 * reads its file again whenever the window comes to the front; a project
 * with no `plans/` folder yet reads its list again on focus until it has one.
 */
export function usePlanWatch(projectId: string | null) {
  const openId = usePlansStore(s => s.open?.id ?? null);
  // Asked again once the project has a canvas: the core does not make `plans/` just to watch it.
  const hasCanvas = usePlansStore(s => Boolean(s.open) || (s.list?.length ?? 0) > 0);
  const [unwatched, setUnwatched] = useState(false);
  useEffect(() => {
    if (!projectId) return;
    let live = true;
    // False: no `plans/` yet. The list reads again on focus (and on an agent's create_canvas) until it has one.
    commands.planWatch(projectId).then(
      watching => live && setUnwatched(!watching),
      (error: unknown) => {
        if (!live) return;
        setUnwatched(true);
        useAppStore.getState().notify({
          status: 'info',
          title: 'Outside changes show when you come back',
          detail: `The canvas folder is not watched (${errorMessage(error)}), so the canvas reads its file again each time the window comes to the front.`,
        });
      }
    );
    return () => {
      live = false;
    };
  }, [projectId, hasCanvas]);
  useEffect(() => {
    if (!unwatched) return;
    const onFocus = () => {
      const { project, open } = usePlansStore.getState();
      if (project) reread(project, open?.id ?? null);
    };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [unwatched]);
  useEffect(() => {
    const stop = events.planChanged.listen(e => {
      const { project, id } = e.payload;
      if (usePlansStore.getState().project === project) reread(project, id);
    });
    // The listener may already be gone with the app (or a test's IPC mock).
    return () => void stop.then(fn => fn()).catch(() => undefined);
  }, []);
  useEffect(() => {
    const project = usePlansStore.getState().project;
    const log = usePlanLogStore.getState();
    const extras = usePlanExtrasStore.getState();
    if (project && openId) {
      void log.load(project, openId);
      void extras.load(project, openId);
    } else {
      log.clear();
      extras.clear();
    }
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
