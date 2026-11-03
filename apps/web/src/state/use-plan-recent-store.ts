/**
 * The rail's recent canvases under Plan, for the open project: its list of
 * canvases and when the person last had each one open. The list is read
 * whole when the project opens and when a canvas the rail does not know
 * changes; a known canvas that changes is read alone (a failed read, a
 * removal, reads the list again); the canvas open now follows the Plan
 * view's own copy, so its autosaves read nothing; and the Plan view's list
 * is taken whenever it is newer. Opening or leaving a canvas, renaming one
 * here and any change while it is open count as seeing it; a canvas edited
 * since is marked. Switching workspace or project starts over with that
 * project's own times, and the Plan view counts only while it shows the
 * rail's workspace and project. Each canvas on the list remembers the read
 * it came from, so a list read that started before a newer read of one
 * canvas answered never puts that canvas's older name or time back.
 */
import { create } from 'zustand';
import { commands, type PlanSummary } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { readJson, writeJson } from '@/lib/storage';
import { CAMERA_PLAN_ID } from './plan-camera';
import { newerOf, renamed, seenFrom, seenKey, seeing, type SeenAt } from './plan-recent';
import { useAppStore } from './use-app-store';
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
  /** The canvas `id` of `project` changed on disk: reads it (or the list) again a moment later. */
  onChanged: (project: string, id: string) => void;
  /** Opens a canvas from the rail: the camera plan with its panel, any other as itself. */
  open: (id: string) => Promise<void>;
}

let timer: ReturnType<typeof setTimeout> | undefined;
/** One timer per known canvas read alone. */
const single = new Map<string, ReturnType<typeof setTimeout>>();
/** Bumped by a switch of workspace or project, so a read answered after it is dropped. */
let generation = 0;
/** Counts every read and patch in the order they started. */
let seq = 0;
/** The read each listed canvas's name and time came from. */
const from = new Map<string, number>();

export const usePlanRecentStore = create<PlanRecentStore>((set, get) => {
  const see = (ids: readonly string[], listed?: readonly PlanSummary[]) => {
    const { root, project, seen } = get();
    if (!root || !project || (!ids.length && !listed)) return;
    const next = seeing(seen, ids, clock.nowMs(), listed);
    set({ seen: next });
    writeJson(seenKey(root, project), next);
  };
  /** Whether a read started at `gen` for `project` and `root` still speaks for the rail. */
  const current = (gen: number, project: string, root: string | null) =>
    gen === generation && get().project === project && get().root === root;
  const read = async () => {
    const { project, root } = get();
    if (!project) return;
    const gen = generation;
    const mine = (seq += 1);
    try {
      const list = await commands.planList(project);
      if (current(gen, project, root)) take(newerOf(get().list, list, from, mine));
    } catch {
      // The rail keeps what it had; the Plan view says why the list could not be read.
    }
  };
  /** Reads one known canvas and puts its name and edited time on the list; a failure reads the list. */
  const readOne = async (id: string) => {
    const { project, root } = get();
    if (!project) return;
    const gen = generation;
    const mine = (seq += 1);
    try {
      const doc = await commands.planGet(project, id);
      if (current(gen, project, root)) patch(id, doc.plan.name, doc.plan.edited, mine);
    } catch {
      if (current(gen, project, root)) void read();
    }
  };
  /**
   * Puts a canvas's latest name and edited time on the list, as a rename
   * here or the open canvas's save, unless a newer read already put its own.
   */
  const patch = (id: string, name: string, edited: string, mine = (seq += 1)) => {
    const list = get().list;
    const at = list.findIndex(p => p.id === id);
    if (at < 0) {
      // A canvas the rail has not listed yet, the camera plan just made among them: read the list.
      clearTimeout(timer);
      timer = setTimeout(() => void read(), RECENT_REREAD_MS);
      return;
    }
    if ((from.get(id) ?? 0) > mine) return;
    from.set(id, mine);
    const was = list[at]!;
    if (was.name === name && was.edited === edited) return;
    const next = list.map((p, i) => (i === at ? { ...p, name, edited } : p));
    take(next);
  };
  const take = (list: PlanSummary[]) => {
    const before = get().list;
    set({ list });
    // A rename here moves the canvas's edited time; it is not a change to mark.
    // Nor is a change to the canvas open now: the person sees it as it lands.
    const plans = usePlansStore.getState();
    const open = ours(plans.project) && plans.open ? [plans.open.id] : [];
    see([...renamed(before, list), ...open], list);
  };

  /** Whether the Plan view shows the rail's own workspace and project. */
  const ours = (project: string | null) =>
    project !== null &&
    project === get().project &&
    (useAppStore.getState().workspace?.root ?? null) === get().root;

  // The Plan view's own reads, opens and leaves keep the rail in step without asking again.
  usePlansStore.subscribe((now, prev) => {
    if (!ours(now.project)) return;
    const ids = [now.open?.id, prev.open?.id].filter(
      (id, i, all): id is string => Boolean(id) && all.indexOf(id) === i
    );
    if (now.open?.id !== prev.open?.id) see(ids);
    if (now.list && now.list !== prev.list) take(newerOf(get().list, now.list, from, (seq += 1)));
    // The open canvas's saves and reloads answer with its file: the rail takes its time from there.
    else if (now.open && now.open !== prev.open)
      patch(now.open.id, now.open.plan.name, now.open.plan.edited);
  });

  return {
    root: null,
    project: null,
    list: [],
    seen: {},

    start: (root, project) => {
      if (root === get().root && project === get().project) return;
      clearTimeout(timer);
      for (const t of single.values()) clearTimeout(t);
      single.clear();
      from.clear();
      generation += 1;
      const seen = root && project ? seenFrom(readJson(seenKey(root, project))) : {};
      set({ root, project, list: [], seen });
      void read();
    },

    onChanged: (project, id) => {
      if (project !== get().project) return;
      const plans = usePlansStore.getState();
      // The open canvas: its own autosaves and agents' writes reach the rail through the Plan view.
      if (ours(plans.project) && plans.open?.id === id) return;
      if (get().list.some(p => p.id === id)) {
        clearTimeout(single.get(id));
        single.set(
          id,
          setTimeout(() => {
            single.delete(id);
            void readOne(id);
          }, RECENT_REREAD_MS)
        );
        return;
      }
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
