/**
 * The open canvas's undo and redo for a person's own edits: each entry is
 * the operations one edit made (`state/plan-ops.ts`). Undo applies them
 * turned around, carefully, to the canvas as it is now, so an agent's
 * write that came in between is never undone with it.
 */
import type { PlanOp } from '@/ipc/client';
import { invertOps } from './plan-ops';

/** How many edits undo reaches back. */
export const HISTORY_MAX = 100;

export interface History {
  past: PlanOp[][];
  future: PlanOp[][];
}

export const EMPTY_HISTORY: History = { past: [], future: [] };

/** `history` with a new edit: it can be undone, and what was undone can no longer be redone. */
export function record(history: History, ops: PlanOp[]): History {
  if (ops.length === 0) return history;
  return { past: [...history.past, ops].slice(-HISTORY_MAX), future: [] };
}

/** The operations an undo applies, and the history after it; null when there is nothing to undo. */
export function takeUndo(history: History): { ops: PlanOp[]; history: History } | null {
  const last = history.past.at(-1);
  if (!last) return null;
  return {
    ops: invertOps(last),
    history: { past: history.past.slice(0, -1), future: [...history.future, last] },
  };
}

/** The operations a redo applies, and the history after it; null when there is nothing to redo. */
export function takeRedo(history: History): { ops: PlanOp[]; history: History } | null {
  const next = history.future.at(-1);
  if (!next) return null;
  return {
    ops: next,
    history: { past: [...history.past, next], future: history.future.slice(0, -1) },
  };
}
