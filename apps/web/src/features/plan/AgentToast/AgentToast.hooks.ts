import { runIds } from '@/state/plan-log';
import { usePlanLogStore } from '@/state/use-plan-log-store';

/**
 * The toast an agent's write shows on the open canvas: who changed it and
 * what, with Undo run (every change of that run not undone yet) and
 * Changes to see them all. It goes when dismissed, when the panel opens or
 * when the run is undone.
 */
export function useAgentToast() {
  const toast = usePlanLogStore(s => s.toast);
  const lines = usePlanLogStore(s => s.lines);
  const dismiss = usePlanLogStore(s => s.dismissToast);
  const setPanel = usePlanLogStore(s => s.setPanel);
  const undoLines = usePlanLogStore(s => s.undo);
  const busy = usePlanLogStore(s => s.undoing);
  const ids = toast ? runIds(lines, toast) : [];
  return {
    toast,
    changes: ids.length,
    busy,
    dismiss,
    show: () => setPanel(true),
    undoRun: () => void undoLines(ids),
  };
}
