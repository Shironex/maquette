/**
 * The open canvas's extras: the agents' removal proposals waiting for the
 * person (read with the change log), answering one (Keep it or Remove), the
 * "Auto layout after agents" switch, marking a mechanics overlap as fine,
 * the Overlaps panel and starting a canvas from a node. Each write saves the
 * person's edits first, then asks the core, then reads the canvas and its
 * log again, as an undo from the log does, so nothing unsaved is lost.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RemovalProposal } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { answerFirst } from './plans-sync';
import { usePlanLogStore } from './use-plan-log-store';
import { usePlansStore } from './use-plans-store';

interface PlanExtrasStore {
  /** `project/canvas` the proposals belong to. */
  key: string | null;
  proposals: RemovalProposal[];
  /** A write is on its way; the next waits for it. */
  busy: boolean;
  /** The Overlaps panel is open. */
  overlaps: boolean;
  load: (project: string, id: string) => Promise<void>;
  /** Answers a proposal: `remove` takes its nodes off as the person's change, else Keep it. */
  answer: (proposal: string, remove: boolean) => Promise<boolean>;
  setAutoLayout: (on: boolean) => Promise<boolean>;
  markFine: (a: string, b: string) => Promise<boolean>;
  /** Makes a canvas from the node and the nodes linked to it and opens it; its id, or null. */
  startFrom: (node: string) => Promise<string | null>;
  setOverlaps: (open: boolean) => void;
  clear: () => void;
}

let turn = 0;

const failed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

export const usePlanExtrasStore = create<PlanExtrasStore>((set, get) => {
  /**
   * Saves the person's edits, runs `job` on the open canvas, then reads the
   * canvas, its log and the proposals again; false when it did not run.
   */
  const write = async (
    title: string,
    job: (project: string, id: string, created: string) => Promise<unknown>
  ): Promise<boolean> => {
    const plans = usePlansStore.getState();
    const { project, open } = plans;
    if (!project || !open || get().busy) return false;
    // Saving now would choose Keep mine for the person: the outside change is answered first.
    if (plans.outside) return answerFirst();
    if (!(await plans.save())) return false;
    set({ busy: true });
    try {
      // The opened canvas's `created`, so an answer never lands on another canvas that took the id.
      await job(project, open.id, open.plan.created);
    } catch (error) {
      failed(title, error);
      set({ busy: false });
      await get().load(project, open.id);
      return false;
    }
    set({ busy: false });
    await usePlansStore.getState().reload();
    await usePlanLogStore.getState().load(project, open.id);
    await get().load(project, open.id);
    return true;
  };

  return {
    key: null,
    proposals: [],
    busy: false,
    overlaps: false,

    load: async (project, id) => {
      const key = `${project}/${id}`;
      if (get().key !== key) set({ key, proposals: [], overlaps: false });
      const mine = (turn += 1);
      try {
        const proposals = await commands.planProposals(project, id);
        if (mine === turn && get().key === key) set({ proposals });
      } catch {
        // The change log's own read says when the log cannot be read.
      }
    },

    answer: (proposal, remove) => {
      // The links the panel showed: the core refuses Remove when they changed since.
      const links = get().proposals.find(p => p.id === proposal)?.links ?? null;
      return write(
        remove ? 'The nodes were not removed' : 'The proposal was not answered',
        (p, id, created) => commands.planAnswerRemoval(p, id, created, proposal, remove, links)
      );
    },

    setAutoLayout: on =>
      write('Auto layout after agents was not switched', (p, id, created) =>
        commands.planSetAutoLayout(p, id, created, on)
      ),

    markFine: (a, b) =>
      write('The overlap was not marked as fine', (p, id, created) =>
        commands.planMarkFine(p, id, created, a, b)
      ),

    startFrom: async node => {
      const plans = usePlansStore.getState();
      const { project, open } = plans;
      if (!project || !open) return null;
      if (plans.outside) {
        answerFirst();
        return null;
      }
      if (!(await plans.save())) return null;
      try {
        const doc = await commands.planStartFrom(project, open.id, node, null);
        await usePlansStore.getState().openCanvas(doc.id);
        await usePlansStore.getState().loadList(project);
        usePlansStore.getState().setArrived(doc.plan.nodes.map(n => n.id));
        return doc.id;
      } catch (error) {
        failed('The canvas was not started', error);
        return null;
      }
    },

    setOverlaps: overlaps => {
      set({ overlaps });
      // One side panel at a time: the Overlaps panel takes the Changes panel's place.
      if (overlaps) usePlanLogStore.getState().setPanel(false);
    },

    clear: () => {
      turn += 1;
      set({ key: null, proposals: [], busy: false, overlaps: false });
    },
  };
});
