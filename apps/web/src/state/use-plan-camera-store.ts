/**
 * The camera plan beside its canvas: the core's view of it (posters, the
 * main line Apply writes, proposals, whether `layout.json` changed since the
 * plan was based on it), the live review of what Apply would write, and
 * the person's actions: Apply through a review that refuses a stale one,
 * undo of an Apply, and compare, rebase or start over after `layout.json`
 * changed. Each action saves the person's edits first, and nothing here
 * writes `layout.json` but Apply and its undo.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type CameraReview,
  type CameraView,
  type RebaseMode,
  type RebaseReport,
} from '@/ipc/client';
import { CAMERA_PLAN_ID } from './plan-camera';
import { useAppStore } from './use-app-store';
import { usePlanLogStore } from './use-plan-log-store';
import { usePlansStore } from './use-plans-store';
import { answerFirst } from './plans-sync';

/** The Apply review: what it would write, and why it was refused when it was. */
export interface ApplyAsk {
  review: CameraReview;
  /** The refusal, when Apply met a changed `layout.json` or plan. */
  error: string | null;
  applying: boolean;
}

/** Compare after an outside change: what a rebase would take, keep and not settle. */
export interface CompareAsk {
  report: RebaseReport;
  busy: boolean;
  /** The refusal, when `layout.json` changed again after the comparison was shown. */
  error: string | null;
}

interface PlanCameraStore {
  project: string | null;
  view: CameraView | null;
  /** The live review of the main line, for the panel's order and counts. */
  review: CameraReview | null;
  error: string | null;
  apply: ApplyAsk | null;
  compare: CompareAsk | null;
  /** Makes the camera plan when the project has none, then opens it. */
  open: (project: string) => Promise<boolean>;
  /** Reads the view and the live review again. */
  refresh: (project: string) => Promise<void>;
  /** Saves the person's edits and shows the Apply review, read fresh. */
  askApply: () => Promise<void>;
  /** Applies the review shown; false and the refusal shown when it was stale. */
  confirmApply: () => Promise<boolean>;
  /** Reads the review again after a refusal. */
  reviewAgain: () => Promise<void>;
  closeApply: () => void;
  /** Undoes the Apply logged as `change`, while `layout.json` is as it left it. */
  undoApply: (change: string) => Promise<boolean>;
  askCompare: () => Promise<void>;
  rebase: (mode: RebaseMode) => Promise<boolean>;
  closeCompare: () => void;
  clear: () => void;
}

const failed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

/** The person's edits on the file first, and no unanswered clash. */
async function saved(): Promise<string | null> {
  const plans = usePlansStore.getState();
  if (plans.outside) {
    answerFirst();
    return null;
  }
  if (!plans.project || !(await plans.save())) return null;
  return plans.project;
}

/** The canvas, its log and the view again, after the core wrote the plan. */
async function reread(project: string) {
  await usePlansStore.getState().reload();
  await usePlanLogStore.getState().load(project, CAMERA_PLAN_ID);
  await usePlanCameraStore.getState().refresh(project);
}

let turn = 0;

export const usePlanCameraStore = create<PlanCameraStore>((set, get) => ({
  project: null,
  view: null,
  review: null,
  error: null,
  apply: null,
  compare: null,

  open: async project => {
    try {
      const view = await commands.planCamera(project);
      set({ project, view, error: null });
    } catch (error) {
      failed('The camera plan did not open', error);
      return false;
    }
    await usePlansStore.getState().openCanvas(CAMERA_PLAN_ID);
    return usePlansStore.getState().open?.id === CAMERA_PLAN_ID;
  },

  refresh: async project => {
    const mine = (turn += 1);
    try {
      const [view, review] = await Promise.all([
        commands.planCamera(project),
        commands.planCameraReview(project),
      ]);
      if (mine === turn) set({ project, view, review, error: null });
    } catch (error) {
      if (mine === turn) set({ project, error: errorMessage(error) });
    }
  },

  askApply: async () => {
    const project = await saved();
    if (!project) return;
    try {
      const review = await commands.planCameraReview(project);
      set({ apply: { review, error: null, applying: false } });
    } catch (error) {
      failed('The review could not be read', error);
    }
  },

  confirmApply: async () => {
    const ask = get().apply;
    const project = get().project;
    if (!ask || !project) return false;
    set({ apply: { ...ask, applying: true } });
    try {
      // Edits made while the review was open reach the file, and the plan key refuses them.
      await usePlansStore.getState().save();
      const view = await commands.planCameraApply(
        project,
        ask.review.layoutKey,
        ask.review.planKey
      );
      set({ view, apply: null });
    } catch (error) {
      set({ apply: { ...ask, applying: false, error: errorMessage(error) } });
      return false;
    }
    useAppStore.getState().notify({
      status: 'done',
      title: 'Applied to layout.json',
      detail: 'The old layout.json is kept in plans/history; Undo in Changes puts it back.',
    });
    await reread(project);
    return true;
  },

  reviewAgain: async () => {
    const project = await saved();
    if (!project) return;
    try {
      const review = await commands.planCameraReview(project);
      set({ apply: { review, error: null, applying: false } });
    } catch (error) {
      failed('The review could not be read', error);
    }
  },

  closeApply: () => set({ apply: null }),

  undoApply: async change => {
    const project = await saved();
    if (!project) return false;
    try {
      const view = await commands.planCameraUndoApply(project, change);
      set({ view });
    } catch (error) {
      failed('The Apply was not undone', error);
      return false;
    }
    await reread(project);
    return true;
  },

  askCompare: async () => {
    const project = await saved();
    if (!project) return;
    try {
      const done = await commands.planCameraRebase(project, 'rebase', true, null);
      set({ compare: { report: done.report, busy: false, error: null }, view: done.view });
    } catch (error) {
      failed('The plan could not be compared', error);
    }
  },

  rebase: async mode => {
    const project = await saved();
    if (!project) return false;
    const ask = get().compare;
    // The layout the person saw, in Compare or the panel: a newer one refuses the rebase.
    const seen = ask?.report.layoutKey ?? get().view?.layoutKey ?? null;
    if (ask) set({ compare: { ...ask, busy: true, error: null } });
    try {
      const done = await commands.planCameraRebase(project, mode, false, seen);
      set({ view: done.view, compare: null });
    } catch (error) {
      if (ask) {
        set({ compare: { ...ask, busy: false, error: errorMessage(error) } });
        return false;
      }
      void get().refresh(project);
      failed(
        mode === 'rebase' ? 'The plan was not rebased' : 'The plan was not started over',
        error
      );
      return false;
    }
    await reread(project);
    return true;
  },

  closeCompare: () => set({ compare: null }),

  clear: () => {
    turn += 1;
    set({ project: null, view: null, review: null, error: null, apply: null, compare: null });
  },
}));
