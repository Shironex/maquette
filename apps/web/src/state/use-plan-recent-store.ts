/**
 * The rail's recent canvases under Plan, for the open project: its list of
 * canvases (read when the project opens, again a moment after a canvas
 * changes, an agent's write or a sync, and taken from the Plan view
 * whenever its list is newer) and when the person last had each one open.
 * Opening or leaving a canvas, renaming one here and any change while it
 * is open count as seeing it; a canvas edited since is marked. Switching workspace or project starts
 * over with that project's own times.
 */
import { create } from 'zustand';
import { commands, type PlanSummary } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { readJson, writeJson } from '@/lib/storage';
import { CAMERA_PLAN_ID } from './plan-camera';
import { renamed, seenFrom, seenKey, seeing, type SeenAt } from './plan-recent';
import { usePlanCameraStore } from './use-plan-camera-store';
import { usePlansStore } from './use-plans-store';

/** How long after a canvas changed the rail reads the list again, in milliseconds. */
export const RECENT_REREAD_MS = 300;

interface PlanRecentStore {
  root: string | null;
  project: string | null;
  list: PlanSummary[];
  seen: SeenAt;
  /** Follows the open workspace and project; null for either clears the rail. */
  start: (root: string | null, project: string | null) => void;
  /** A canvas of `project` changed on disk: reads the list again a moment later. */
  onChanged: (project: string) => void;
  /** Opens a canvas from the rail: the camera plan with its panel, any other as itself. */
  open: (id: string) => Promise<void>;
}

let timer: ReturnType<typeof setTimeout> | undefined;
/** Counts reads, so one answered after a newer one or a switch is dropped. */
let reads = 0;

export const usePlanRecentStore = create<PlanRecentStore>((set, get) => {
  const see = (ids: readonly string[], listed?: readonly PlanSummary[]) => {
    const { root, project, seen } = get();
    if (!root || !project || (!ids.length && !listed)) return;
    const next = seeing(seen, ids, clock.nowMs(), listed);
    set({ seen: next });
    writeJson(seenKey(root, project), next);
  };
  const read = async () => {
    const { project, root } = get();
    if (!project) return;
    const mine = (reads += 1);
    try {
      const list = await commands.planList(project);
      if (mine === reads && get().project === project && get().root === root) take(list);
    } catch {
      // The rail keeps what it had; the Plan view says why the list could not be read.
    }
  };
  const take = (list: PlanSummary[]) => {
    const before = get().list;
    set({ list });
    // A rename here moves the canvas's edited time; it is not a change to mark.
    // Nor is a change to the canvas open now: the person sees it as it lands.
    const plans = usePlansStore.getState();
    const open = plans.project === get().project && plans.open ? [plans.open.id] : [];
    see([...renamed(before, list), ...open], list);
  };

  // The Plan view's own reads, opens and leaves keep the rail in step without asking again.
  usePlansStore.subscribe((now, prev) => {
    if (now.project !== get().project) return;
    const ids = [now.open?.id, prev.open?.id].filter(
      (id, i, all): id is string => Boolean(id) && all.indexOf(id) === i
    );
    if (now.open?.id !== prev.open?.id) see(ids);
    if (now.list && now.list !== prev.list) take(now.list);
  });

  return {
    root: null,
    project: null,
    list: [],
    seen: {},

    start: (root, project) => {
      if (root === get().root && project === get().project) return;
      clearTimeout(timer);
      reads += 1;
      const seen = root && project ? seenFrom(readJson(seenKey(root, project))) : {};
      set({ root, project, list: [], seen });
      void read();
    },

    onChanged: project => {
      if (project !== get().project) return;
      clearTimeout(timer);
      timer = setTimeout(() => void read(), RECENT_REREAD_MS);
    },

    open: async id => {
      const project = get().project;
      if (!project) return;
      const plans = usePlansStore.getState();
      if (plans.open?.id === id && plans.project === project) return;
      if (plans.project !== project) await plans.loadList(project);
      if (usePlansStore.getState().project !== project) return;
      const camera = get().list.some(p => p.id === id && p.camera);
      if (camera && id === CAMERA_PLAN_ID) await usePlanCameraStore.getState().open(project);
      else await usePlansStore.getState().openCanvas(id);
    },
  };
});
