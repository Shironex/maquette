/**
 * The open canvas's change log for the Changes panel, the toast an agent's
 * write shows and the count on the Changes button. It reads again when the
 * canvas opens and whenever its file or log changes. Undo from the log
 * saves the person's edits first, then undoes in the core and reloads the
 * canvas, so nothing unsaved is lost.
 */
import { create } from 'zustand';
import { commands, errorMessage, type PlanLogLine } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { freshAgentLine, type LogFilter } from './plan-log';
import { useAppStore } from './use-app-store';
import { usePlansStore } from './use-plans-store';

/** How many log lines the panel reads. */
export const LOG_LIMIT = 200;

interface PlanLogStore {
  /** `project/canvas` the lines belong to. */
  key: string | null;
  lines: PlanLogLine[];
  error: string | null;
  panel: boolean;
  filter: LogFilter;
  /** Agent changes after this time count as new (RFC 3339): when the canvas opened or the panel last did. */
  seenAt: string;
  /** The agent change the toast shows, until dismissed. */
  toast: PlanLogLine | null;
  dismissed: string | null;
  /** An undo from the log is on its way; the next waits for it. */
  undoing: boolean;
  load: (project: string, id: string) => Promise<void>;
  undo: (lines: string[]) => Promise<boolean>;
  setPanel: (open: boolean) => void;
  setFilter: (filter: LogFilter) => void;
  dismissToast: () => void;
  clear: () => void;
}

let turn = 0;

const initial = {
  key: null,
  lines: [],
  error: null,
  panel: false,
  filter: 'all' as LogFilter,
  toast: null,
  dismissed: null,
  undoing: false,
};

export const usePlanLogStore = create<PlanLogStore>((set, get) => ({
  ...initial,
  seenAt: '',

  load: async (project, id) => {
    const key = `${project}/${id}`;
    if (get().key !== key) set({ ...initial, key, seenAt: clock.now().toISOString() });
    const mine = (turn += 1);
    try {
      const lines = await commands.planLog(project, id, LOG_LIMIT);
      if (mine !== turn || get().key !== key) return;
      const fresh = freshAgentLine(lines, get().seenAt);
      const toast = fresh && fresh.id !== get().dismissed && !get().panel ? fresh : null;
      set({ lines, error: null, toast });
    } catch (error) {
      if (mine === turn) set({ error: errorMessage(error) });
    }
  },

  undo: async ids => {
    const plans = usePlansStore.getState();
    const { project, open } = plans;
    if (!project || !open || ids.length === 0) return false;
    // The person's edits reach the file first, so the undo meets them there.
    if (!(await plans.save())) return false;
    try {
      await commands.planUndo(project, open.id, ids);
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'The change was not undone',
        detail: errorMessage(error),
      });
      return false;
    }
    set({ toast: null });
    await usePlansStore.getState().reload();
    await get().load(project, open.id);
    return true;
  },

  setPanel: panel =>
    set(panel ? { panel, toast: null, seenAt: clock.now().toISOString() } : { panel }),

  setFilter: filter => set({ filter }),

  dismissToast: () => set(s => ({ toast: null, dismissed: s.toast?.id ?? s.dismissed })),

  clear: () => {
    turn += 1;
    set({ ...initial, seenAt: '' });
  },
}));
