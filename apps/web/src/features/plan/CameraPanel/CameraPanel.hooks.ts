import { useEffect } from 'react';
import { events } from '@/ipc/generated/bindings';
import { clock } from '@/lib/clock';
import { adopted, discarded, MAX_STOPS, withNewStop } from '@/state/plan-camera';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';

/** How long the view waits after the canvas changed before reading the core's view again. */
export const VIEW_REREAD_MS = 250;

/**
 * The camera plan's side panel: the core's view read again whenever the
 * canvas file changes (a save, an agent, an undo), `layout.json` changes
 * (the Inspector, an agent's set_layout, an Apply) or the window comes
 * back; and the panel's actions on the flow (add a stop, take or discard
 * a proposal) and on the plan (compare, rebase, start over, Apply).
 */
export function useCameraPanel() {
  const project = usePlansStore(s => s.project);
  const open = usePlansStore(s => s.open);
  const view = usePlanCameraStore(s => s.view);
  const review = usePlanCameraStore(s => s.review);
  const error = usePlanCameraStore(s => s.error);
  const refresh = usePlanCameraStore(s => s.refresh);
  const askApply = usePlanCameraStore(s => s.askApply);
  const askCompare = usePlanCameraStore(s => s.askCompare);
  const rebase = usePlanCameraStore(s => s.rebase);
  const replaceFlow = usePlansStore(s => s.replaceFlow);
  const setArrived = usePlansStore(s => s.setArrived);
  const selected = usePlansStore(
    s => s.nodes.find(n => n.selected && n.data.node.stop)?.id ?? null
  );

  useEffect(() => {
    if (!project || !open) return;
    const timer = setTimeout(() => void refresh(project), VIEW_REREAD_MS);
    return () => clearTimeout(timer);
  }, [project, open, refresh]);
  // Leaving the camera plan forgets its view.
  useEffect(() => () => usePlanCameraStore.getState().clear(), []);
  useEffect(() => {
    if (!project) return;
    const again = () => void refresh(project);
    window.addEventListener('focus', again);
    const stop = events.layoutChanged.listen(e => {
      if (e.payload.project === project) again();
    });
    return () => {
      window.removeEventListener('focus', again);
      // The listener may already be gone with the app (or a test's IPC mock).
      void stop.then(fn => fn()).catch(() => undefined);
    };
  }, [project, refresh]);

  const main = view?.main ?? [];
  const proposals = (view?.lines ?? []).filter(l => l.agent !== null && l.nodes.length > 0);
  const problems = (review?.problems ?? []).filter(p => !p.startsWith('Nothing to apply'));
  const d = review?.diff;
  const same = Boolean(d && d.added + d.removed + d.moved + d.changed === 0);
  const flow = () => usePlansStore.getState();
  return {
    view,
    review,
    error,
    main,
    proposals,
    problems,
    stale: Boolean(view?.stale),
    selected,
    /** Why Apply is off, or null when it can go. */
    applyOff: !review
      ? 'Reading the plan...'
      : view?.stale
        ? 'layout.json changed since this plan was based on it: compare or rebase first'
        : problems.length
          ? problems[0]!
          : same
            ? 'layout.json already has this order and these settings'
            : null,
    canAdd: main.length < MAX_STOPS,
    addStop: () => {
      const { nodes, edges } = flow();
      const next = withNewStop(nodes, edges, main, clock.nowMs());
      replaceFlow(next.nodes, next.edges);
      setArrived([next.id]);
    },
    adopt: (line: string[]) => {
      const { nodes, edges } = flow();
      const next = adopted(nodes, edges, main, line, clock.nowMs());
      replaceFlow(next.nodes, next.edges);
    },
    discard: (line: string[]) => {
      const { nodes, edges } = flow();
      const next = discarded(nodes, edges, line);
      replaceFlow(next.nodes, next.edges);
    },
    apply: () => void askApply(),
    compare: () => void askCompare(),
    rebase: () => void rebase('rebase'),
    startOver: () => void rebase('start-over'),
  };
}
