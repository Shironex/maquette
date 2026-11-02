/**
 * The open canvas against its file: saving, reloading after an outside
 * change, and undo and redo.
 *
 * `open` is the canvas as the file last held it (the base). A save sends
 * only the operations between the base and what the flow shows, and the
 * core lays them over the file as it is now, so an agent's write made
 * meanwhile stays. Its answer becomes the new base, with any edits made
 * while it saved laid over it again. A file changed outside (an agent,
 * another window, a sync from another computer) is read again: when the
 * person has no unsaved edits it simply shows; when they do, their edits
 * are laid over it, and only when both changed the same thing does
 * `outside` ask which side wins. Until the person answers nothing saves:
 * not autosave, leaving, quitting, renaming, opening a link or an undo
 * from the log, since any save would choose Keep mine for them. The app's
 * own writes read back as the base, so they never reload in a loop.
 */
import {
  commands,
  errorMessage,
  type PlanDoc,
  type PlanFile,
  type PlanNode,
  type PlanOp,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { useAppStore } from './use-app-store';
import { EMPTY_HISTORY, record, takeRedo, takeUndo } from './plan-history';
import { applyOps, conflictsOf, diffPlans, rebase } from './plan-ops';
import { flowFrom, planOf, samePlan } from './plans';
import type { GetPlans, PlansStore, SetPlans } from './plans-store.types';

/** How long after the last change the canvas saves, in milliseconds. */
export const SAVE_DELAY = 500;

export const SAVE_FAILED = 'The canvas was not saved';
/** Why nothing saves while the outside change waits for its answer. */
export const ANSWER_FIRST = 'The canvas changed outside';
export const ANSWER_FIRST_DETAIL =
  'Choose Keep mine or Use theirs first (Answer clash in the top bar). Nothing saves until you answer.';

let timer: ReturnType<typeof setTimeout> | undefined;
/** The save on its way, so the next one waits for it instead of racing it. */
let inflight: Promise<boolean> | null = null;
/** Counts saves started, so a reload knows one ran while it read the file. */
let saves = 0;
/** A reload saves kept overtaking, tried again a moment later. */
let retry: ReturnType<typeof setTimeout> | undefined;
/** How long a reload that saves kept overtaking waits before it reads again, in milliseconds. */
export const RELOAD_RETRY_MS = 1000;
/** Counts renames, so a save answered after a rename keeps the new name. */
let renames = 0;
/** The flow's canvas at the last undo step, so the next edit records what changed since. */
let mark: PlanFile | null = null;

export const noteRename = () => {
  renames += 1;
};

/** Whether an outside write changed what `before` says (its text or links), not only where it sits. */
const editedOutside = (before: PlanNode, after: PlanNode) =>
  !deepEqual(
    { ...before, x: 0, y: 0, personWrote: false },
    { ...after, x: 0, y: 0, personWrote: false }
  );

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });
/** Says the outside change needs its answer before anything saves; false, for the path it stops. */
export const answerFirst = () => {
  useAppStore
    .getState()
    .notify({ status: 'info', title: ANSWER_FIRST, detail: ANSWER_FIRST_DETAIL });
  return false;
};

export function createSync(set: SetPlans, get: GetPlans) {
  /** The canvas the flow shows, as the file would hold it. */
  const draft = () => {
    const { open, nodes, edges } = get();
    return open ? planOf(open.plan, nodes, edges) : null;
  };
  /** Whether the open canvas differs from its file. */
  const pending = () => {
    const next = draft();
    const open = get().open;
    return Boolean(next && open && !samePlan(next, open.plan));
  };
  /** Records the person's edit since the last step, for undo. */
  const checkpoint = () => {
    const now = draft();
    if (!now) return;
    if (mark) {
      const ops = diffPlans(mark, now);
      if (ops.length) set({ history: record(get().history, ops) });
    }
    mark = now;
  };
  /** After a change in the flow: an undo step (unless mid drag) and a save half a second later. */
  const changed = (step = true) => {
    if (step) checkpoint();
    const dirty = pending();
    if (get().dirty !== dirty) set({ dirty });
    clearTimeout(timer);
    if (!dirty || get().outside) return;
    timer = setTimeout(() => void get().save(), SAVE_DELAY);
  };
  /**
   * Shows `local` as the flow over the base `doc`. With `arrive` (a write
   * from outside), nodes new to the flow arrive and the ones it added or
   * edited get their mark.
   */
  const adopt = (doc: PlanDoc, local: PlanFile, arrive = true) => {
    const prev = draft();
    const { nodes, edges, images } = get();
    const had = new Map(nodes.map(n => [n.id, n.data.node]));
    const flow = flowFrom(local, nodes, edges);
    const marks = { ...get().marks };
    const came: string[] = [];
    if (arrive) {
      for (const node of local.nodes) {
        const before = had.get(node.id);
        if (!before) {
          came.push(node.id);
          marks[node.id] = 'added';
        } else if (!marks[node.id] && editedOutside(before, node)) marks[node.id] = 'edited';
      }
    }
    set({
      open: doc,
      ...flow,
      images: { ...images, ...doc.images },
      marks,
      ...(came.length ? { arrived: came } : {}),
    });
    // The last undo step (a drag's start, too) takes in what changed, so the next step records only the person's edit.
    mark = mark && prev ? applyOps(mark, diffPlans(prev, local), 'mine').plan : local;
  };
  /** Forgets the undo steps and timer, for a canvas opened or closed. */
  const reset = (plan: PlanFile | null) => {
    clearTimeout(timer);
    clearTimeout(retry);
    mark = plan;
    set({ history: EMPTY_HISTORY, outside: null, arrived: [], marks: {} });
  };
  /** Saves until the file holds every edit; false when a save fails. */
  const flush = async () => {
    for (let round = 0; round < 5; round += 1) {
      if (!(await get().save(true))) return false;
      if (!pending()) return true;
    }
    return !pending();
  };
  /** Applies `ops` to the flow carefully (undo, redo) and saves. */
  const step = (ops: PlanOp[]) => {
    const now = draft();
    const open = get().open;
    if (!now || !open) return;
    const next = applyOps(now, ops, 'careful').plan;
    adopt(open, next, false);
    changed(false);
  };
  /** Lays the person's unsaved edits over `doc`, the file as changed outside. */
  const settle = (doc: PlanDoc, winner: 'mine' | 'theirs') => {
    const open = get().open;
    const local = draft();
    if (!open || !local) return;
    adopt(doc, rebase(open.plan, local, doc.plan, winner));
    set({ outside: null });
    changed(false);
  };

  const actions: Pick<PlansStore, 'save' | 'reload' | 'keepMine' | 'useTheirs' | 'undo' | 'redo'> =
    {
      save: async (quiet = false) => {
        clearTimeout(timer);
        // One save at a time: wait for the one on its way, then save what is newest.
        while (inflight) await inflight;
        const { project, open } = get();
        const next = draft();
        if (!project || !open || !next || samePlan(next, open.plan)) return true;
        // Saving now would choose Keep mine for the person: every path waits for their answer.
        if (get().outside) {
          if (!quiet) answerFirst();
          return false;
        }
        saves += 1;
        const ops = diffPlans(open.plan, next);
        const sent = renames;
        const run = (async () => {
          set({ saving: true });
          try {
            const saved = await commands.planApply(project, open.id, ops);
            const now = get().open;
            if (now?.id !== open.id) {
              set({ saving: false });
              return true;
            }
            const name = sent === renames ? saved.plan.name : now.plan.name;
            const doc = { ...saved, plan: { ...saved.plan, name } };
            // Edits made while it saved go over the file's answer, which may hold an agent's write too.
            const later = diffPlans(next, draft() ?? next);
            const local = applyOps(doc.plan, later, 'mine').plan;
            if (samePlan(local, draft() ?? next)) set({ open: doc });
            else adopt(doc, local);
            set({ images: { ...get().images, ...doc.images }, saving: false, saveError: null });
            changed(false);
            return true;
          } catch (error) {
            set({ saving: false, saveError: errorMessage(error), dirty: true });
            if (!quiet) notifyFailed(SAVE_FAILED, error);
            return false;
          } finally {
            inflight = null;
          }
        })();
        inflight = run;
        return run;
      },

      reload: async () => {
        // The app's own save first: its answer is the base the file is compared with.
        clearTimeout(retry);
        let doc: PlanDoc | null = null;
        // A save that ran while the file was read makes the read stale: read again after it.
        for (let round = 0; round < 5 && !doc; round += 1) {
          while (inflight) await inflight;
          const { project, open } = get();
          if (!project || !open) return;
          const started = saves;
          try {
            doc = await commands.planGet(project, open.id);
          } catch {
            // Gone or unreadable for a moment (a sync mid write): the next change reads it again.
            return;
          }
          if (started !== saves || inflight) doc = null;
        }
        if (!doc) {
          // Saves kept landing while it read: read again once they settle.
          retry = setTimeout(() => void get().reload(), RELOAD_RETRY_MS);
          return;
        }
        const base = get().open;
        const local = draft();
        if (base?.id !== doc.id || !local) return;
        if (samePlan(doc.plan, base.plan)) {
          // The file matches the base again: a pending clash names a version the file does not hold.
          if (get().outside) {
            set({ outside: null });
            changed(false);
          }
          return;
        }
        if (samePlan(local, base.plan)) {
          adopt(doc, doc.plan);
          set({ outside: null });
          changed(false);
          return;
        }
        const conflicts = conflictsOf(base.plan, local, doc.plan);
        if (conflicts.length === 0) {
          settle(doc, 'mine');
          return;
        }
        clearTimeout(timer);
        set({ outside: { doc, conflicts } });
      },

      keepMine: () => {
        const outside = get().outside;
        if (outside) settle(outside.doc, 'mine');
      },

      useTheirs: () => {
        const outside = get().outside;
        if (outside) settle(outside.doc, 'theirs');
      },

      undo: () => {
        checkpoint();
        const taken = takeUndo(get().history);
        if (!taken) return;
        set({ history: taken.history });
        step(taken.ops);
      },

      redo: () => {
        const taken = takeRedo(get().history);
        if (!taken) return;
        set({ history: taken.history });
        step(taken.ops);
      },
    };

  return { draft, pending, changed, adopt, reset, flush, actions };
}
