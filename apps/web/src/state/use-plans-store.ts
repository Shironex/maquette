/**
 * The Plan view's state: the project's canvases, the one open and its
 * nodes and edges as React Flow holds them. Every change that alters the
 * file saves it half a second later; selecting and measuring do not.
 * Leaving a canvas (Back, another canvas, another project, quitting) waits
 * for a save on its way and saves the latest edits first. When that save
 * fails the canvas stays open and `leaving` asks whether to try again or
 * leave without saving; nothing is dropped without that answer.
 */
import { applyEdgeChanges, applyNodeChanges, type Connection } from '@xyflow/react';
import type { EdgeChange, NodeChange } from '@xyflow/react';
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type NodeKind,
  type PlanDoc,
  type PlanSummary,
} from '@/ipc/client';
import { clock } from '@/lib/clock';
import {
  flowEdgesOf,
  flowNodeOf,
  flowNodesOf,
  newEdge,
  newNode,
  planOf,
  samePlan,
  withFields,
  type FlowEdge,
  type FlowNode,
  type NodeFields,
} from './plans';
import { useAppStore } from './use-app-store';

/** How long after the last change the canvas saves, in milliseconds. */
export const SAVE_DELAY = 500;

/** Where the person was going when a save failed on the way out. */
export type Leave =
  { to: 'list' } | { to: 'canvas'; id: string } | { to: 'project'; project: string };

interface PlansStore {
  project: string | null;
  list: PlanSummary[] | null;
  listError: string | null;
  /** The open canvas as the file last held it. */
  open: PlanDoc | null;
  opening: string | null;
  openError: string | null;
  nodes: FlowNode[];
  edges: FlowEdge[];
  /** A picture's stored path to the file on this computer. */
  images: Record<string, string>;
  saving: boolean;
  saveError: string | null;
  /** The open canvas holds edits the file does not have yet. */
  dirty: boolean;
  /** A save failed on the way out; the canvas stays open until answered. */
  leaving: Leave | null;
  /** A note just made, which opens for writing. */
  fresh: string | null;
  clearFresh: () => void;
  loadList: (project: string) => Promise<void>;
  /** Makes a canvas and opens it; its id, or null when refused. */
  create: (name: string) => Promise<string | null>;
  rename: (id: string, name: string) => Promise<boolean>;
  remove: (id: string) => Promise<boolean>;
  openCanvas: (id: string) => Promise<void>;
  /** Back to the list, saving first; false (and `leaving` set) when the save failed. */
  close: () => Promise<boolean>;
  /** After a failed save on the way out: save again and go on when it works. */
  retryLeave: () => Promise<void>;
  /** After a failed save on the way out: drop the unsaved edits and go on. */
  leaveAnyway: () => Promise<void>;
  /** After a failed save on the way out: stay on the canvas. */
  stay: () => void;
  /** Saves every edit for quitting; why it could not, or null. */
  saveForQuit: () => Promise<string | null>;
  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange<FlowEdge>[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (kind: NodeKind, at: { x: number; y: number }, fields?: NodeFields) => string;
  updateNode: (id: string, fields: NodeFields) => void;
  /** Picks a picture and adds it at `at`; false when cancelled or refused. */
  addPicture: (at: { x: number; y: number }) => Promise<boolean>;
  openLink: (node: string) => Promise<void>;
  /** Saves the open canvas now when it changed; `quiet` for a failure a dialog asks about. */
  save: (quiet?: boolean) => Promise<boolean>;
}

let timer: ReturnType<typeof setTimeout> | undefined;
/** The save on its way, so the next one waits for it instead of racing it. */
let inflight: Promise<boolean> | null = null;
/** Counts renames, so a save answered after a rename keeps the new name. */
let renames = 0;

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });
const SAVE_FAILED = 'The canvas was not saved';
/** A dialog asks about the failed save, so a notice saying the same goes. */
const dropSaveNotice = () => {
  const app = useAppStore.getState();
  if (app.notice?.title === SAVE_FAILED) app.dismissNotice();
  return false;
};

export const usePlansStore = create<PlansStore>((set, get) => {
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
  const changed = () => {
    const dirty = pending();
    if (get().dirty !== dirty) set({ dirty });
    if (!dirty) return;
    clearTimeout(timer);
    timer = setTimeout(() => void get().save(), SAVE_DELAY);
  };
  /** Saves until the file holds every edit; false when a save fails, which the caller asks about. */
  const flush = async () => {
    for (let round = 0; round < 5; round += 1) {
      if (!(await get().save(true))) return dropSaveNotice();
      if (!pending()) return true;
    }
    return !pending();
  };
  const shut = () => {
    clearTimeout(timer);
    set({ open: null, nodes: [], edges: [], images: {}, dirty: false, saveError: null });
  };
  /** Leaves the open canvas for `leave` once it is saved; false and asks when it is not. */
  const settle = async (leave: Leave) => {
    if (!get().open || (await flush())) return true;
    set({ leaving: leave });
    return false;
  };
  const go = async (leave: Leave) => {
    if (leave.to === 'list') await get().close();
    else if (leave.to === 'canvas') await get().openCanvas(leave.id);
    else await get().loadList(leave.project);
  };
  const take = (doc: PlanDoc) =>
    set({
      open: doc,
      opening: null,
      nodes: flowNodesOf(doc.plan),
      edges: flowEdgesOf(doc.plan),
      images: doc.images,
      saveError: null,
      dirty: false,
      leaving: null,
    });
  const putSummary = (summary: PlanSummary) =>
    set(s => ({
      list: [...(s.list ?? []).filter(p => p.id !== summary.id), summary].sort((a, b) =>
        a.id.localeCompare(b.id)
      ),
    }));

  return {
    project: null,
    list: null,
    listError: null,
    open: null,
    opening: null,
    openError: null,
    nodes: [],
    edges: [],
    images: {},
    saving: false,
    saveError: null,
    dirty: false,
    leaving: null,
    fresh: null,
    clearFresh: () => set({ fresh: null }),

    loadList: async project => {
      if (get().project !== project) {
        // The canvas of the project being left saves first, or stays open and asks.
        if (!(await settle({ to: 'project', project }))) return;
        shut();
        set({ project, list: null });
      }
      set({ listError: null });
      try {
        const list = await commands.planList(project);
        if (get().project === project) set({ list });
      } catch (error) {
        if (get().project === project) set({ listError: errorMessage(error) });
      }
    },

    create: async name => {
      const project = get().project;
      if (!project || !(await settle({ to: 'list' }))) return null;
      try {
        const doc = await commands.planCreate(project, name);
        take(doc);
        await get().loadList(project);
        return doc.id;
      } catch (error) {
        notifyFailed('The canvas was not made', error);
        return null;
      }
    },

    rename: async (id, name) => {
      const project = get().project;
      if (!project) return false;
      await get().save();
      try {
        const summary = await commands.planRename(project, id, name);
        putSummary(summary);
        renames += 1;
        const open = get().open;
        if (open?.id === id) set({ open: { ...open, plan: { ...open.plan, name: summary.name } } });
        return true;
      } catch (error) {
        notifyFailed('The canvas was not renamed', error);
        return false;
      }
    },

    remove: async id => {
      const project = get().project;
      if (!project) return false;
      try {
        await commands.planRemove(project, id);
        if (get().open?.id === id) shut();
        set(s => ({ list: s.list?.filter(p => p.id !== id) ?? null }));
        return true;
      } catch (error) {
        notifyFailed('The canvas was not removed', error);
        return false;
      }
    },

    openCanvas: async id => {
      const project = get().project;
      if (!project || !(await settle({ to: 'canvas', id }))) return;
      set({ opening: id, openError: null });
      try {
        const doc = await commands.planGet(project, id);
        if (get().opening === id) take(doc);
      } catch (error) {
        if (get().opening === id) set({ opening: null, openError: errorMessage(error) });
      }
    },

    close: async () => {
      if (!(await settle({ to: 'list' }))) return false;
      // The workspace's project, which differs after a switch whose save failed and Stay.
      const project = useAppStore.getState().workspace?.project?.id ?? get().project;
      shut();
      set({ openError: null });
      if (project) await get().loadList(project);
      return true;
    },

    retryLeave: async () => {
      const leave = get().leaving;
      set({ leaving: null });
      if (leave) await go(leave);
    },

    leaveAnyway: async () => {
      const leave = get().leaving;
      set({ leaving: null });
      shut();
      if (leave) await go(leave);
    },

    stay: () => set({ leaving: null }),

    saveForQuit: async () => ((await flush()) ? null : (get().saveError ?? SAVE_FAILED)),

    onNodesChange: changes => {
      set({ nodes: applyNodeChanges(changes, get().nodes) });
      changed();
    },

    onEdgesChange: changes => {
      set({ edges: applyEdgeChanges(changes, get().edges) });
      changed();
    },

    onConnect: connection => {
      const edge = newEdge(connection, clock.nowMs());
      if (!edge) return;
      const duplicate = get().edges.some(
        e =>
          (e.source === edge.source && e.target === edge.target) ||
          (e.source === edge.target && e.target === edge.source)
      );
      if (duplicate) return;
      set({ edges: [...get().edges, edge] });
      changed();
    },

    addNode: (kind, at, fields = {}) => {
      const node = newNode(kind, at, fields, clock.nowMs());
      const nodes = get().nodes.map(n => (n.selected ? { ...n, selected: false } : n));
      set({
        nodes: [...nodes, { ...flowNodeOf(node), selected: true }],
        fresh: kind === 'note' ? node.id : null,
      });
      changed();
      return node.id;
    },

    updateNode: (id, fields) => {
      set({
        nodes: get().nodes.map(n => (n.id === id ? withFields(n, fields) : n)),
      });
      changed();
    },

    addPicture: async at => {
      const project = get().project;
      if (!project) return false;
      try {
        const picked = await commands.planPickImage(project);
        if (!picked) return false;
        set({ images: { ...get().images, [picked.path]: picked.file } });
        get().addNode('image', at, { image: picked.path, title: picked.name });
        return true;
      } catch (error) {
        notifyFailed('The picture was not added', error);
        return false;
      }
    },

    openLink: async node => {
      const { project, open } = get();
      if (!project || !open) return;
      await get().save();
      try {
        await commands.planOpenLink(project, open.id, node);
      } catch (error) {
        notifyFailed('The link did not open', error);
      }
    },

    save: async (quiet = false) => {
      clearTimeout(timer);
      // One save at a time: wait for the one on its way, then save what is newest.
      while (inflight) await inflight;
      const { project, open } = get();
      const next = draft();
      if (!project || !open || !next || samePlan(next, open.plan)) return true;
      const sent = renames;
      const run = (async () => {
        set({ saving: true });
        try {
          const saved = await commands.planSave(project, open.id, next);
          const now = get().open;
          if (now?.id !== open.id) {
            set({ saving: false });
            return true;
          }
          const name = sent === renames ? saved.plan.name : now.plan.name;
          const doc = { ...saved, plan: { ...saved.plan, name } };
          // The core wrote something other than what was sent and nothing changed since: show its canvas.
          const rebuild = !samePlan(doc.plan, next) && samePlan(draft() ?? next, next);
          set({
            open: doc,
            ...(rebuild ? { nodes: flowNodesOf(doc.plan), edges: flowEdgesOf(doc.plan) } : {}),
            images: { ...get().images, ...doc.images },
            saving: false,
            saveError: null,
          });
          // Changes made while it saved go next.
          changed();
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
  };
});
