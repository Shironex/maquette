/**
 * The open canvas's change log for the Changes panel, the toast an agent's
 * write shows and the count on the Changes button. It reads again when the
 * canvas opens and whenever its file or log changes. Undo from the log
 * saves the person's edits first, then undoes in the core and reloads the
 * canvas, so nothing unsaved is lost.
 */
import { create } from 'zustand';
import { commands, errorMessage, type KeepNeed, type PlanLogLine } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { freshAgentLine, type LogFilter } from './plan-log';
import { useAppStore } from './use-app-store';
import { answerFirst } from './plans-sync';
import { usePlansStore } from './use-plans-store';

/** What an undo says when everything it would undo was changed since. */
export const NOTHING_TO_UNDO = 'Already changed, nothing to undo';

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
  /**
   * Keeps (accepts) agent changes: the log stops holding room to undo them.
   * When earlier changes sit under them, it asks first (`keepAsk`).
   */
  keep: (lines: string[]) => Promise<boolean>;
  /** A Keep waiting for the person: the changes named and the earlier ones it needs too. */
  keepAsk: { ids: string[]; needs: KeepNeed[] } | null;
  /** Keeps the asked changes with every earlier one listed. */
  keepAll: () => Promise<boolean>;
  cancelKeep: () => void;
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
  keepAsk: null,
};

const notKept = (error: unknown) =>
  useAppStore.getState().notify({
    status: 'failed',
    title: 'The change was not kept',
    detail: errorMessage(error),
  });

/** Keeps the `clicked` changes with the earlier ones the person `confirmed`, exactly as listed. */
async function keepNamed(project: string, canvas: string, clicked: string[], confirmed: string[]) {
  try {
    await commands.planKeep(project, canvas, clicked, confirmed);
    return true;
  } catch (error) {
    notKept(error);
    return false;
  }
}

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
    // Saving now would choose Keep mine for the person: the outside change is answered first.
    if (plans.outside) return answerFirst();
    // The person's edits reach the file first, so the undo meets them there.
    if (!(await plans.save())) return false;
    try {
      const done = await commands.planUndo(project, open.id, ids);
      if (!done.changed)
        useAppStore.getState().notify({
          status: 'info',
          title: NOTHING_TO_UNDO,
          detail:
            'What this change touched was edited or removed since, so nothing changed. It counts as undone and holds no more room in the log.',
        });
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

  keep: async ids => {
    const { project, open } = usePlansStore.getState();
    if (!project || !open || ids.length === 0) return false;
    let needs: KeepNeed[];
    try {
      needs = await commands.planKeepNeeds(project, open.id, ids);
    } catch (error) {
      notKept(error);
      return false;
    }
    // Nothing is kept that the person did not see: earlier changes wait for a yes.
    if (needs.length) {
      set({ keepAsk: { ids, needs } });
      return false;
    }
    if (!(await keepNamed(project, open.id, ids, []))) return false;
    set({ toast: null });
    await get().load(project, open.id);
    return true;
  },

  keepAll: async () => {
    const ask = get().keepAsk;
    const { project, open } = usePlansStore.getState();
    set({ keepAsk: null });
    if (!ask || !project || !open) return false;
    const confirmed = ask.needs.map(n => n.id);
    if (!(await keepNamed(project, open.id, ask.ids, confirmed))) {
      // The list moved under the dialog: the next Keep asks again with what is there now.
      await get().load(project, open.id);
      return false;
    }
    set({ toast: null });
    await get().load(project, open.id);
    return true;
  },

  cancelKeep: () => set({ keepAsk: null }),

  setPanel: panel =>
    set(panel ? { panel, toast: null, seenAt: clock.now().toISOString() } : { panel }),

  setFilter: filter => set({ filter }),

  dismissToast: () => set(s => ({ toast: null, dismissed: s.toast?.id ?? s.dismissed })),

  clear: () => {
    turn += 1;
    set({ ...initial, seenAt: '' });
  },
}));
