/**
 * The Plan view's state: the project's canvases, the one open and its
 * nodes and edges as React Flow holds them. Every change that alters the
 * file saves it half a second later; selecting and measuring do not.
 * Saving, reloading after an outside change and undo live in
 * `plans-sync.ts`. Leaving a canvas (Back, another canvas, another project,
 * quitting) waits for a save on its way and saves the latest edits first.
 * When that save fails the canvas stays open and `leaving` asks whether to
 * try again or leave without saving; nothing is dropped without that answer.
 * While an outside change waits for Keep mine or Use theirs, leaving,
 * quitting and renaming wait for that answer too.
 */
import { applyEdgeChanges, applyNodeChanges } from '@xyflow/react';
import { create } from 'zustand';
import { commands, errorMessage, type PlanDoc, type PlanSummary } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { healed } from './plan-camera';
import { EMPTY_HISTORY } from './plan-history';
import { flowEdgesOf, flowNodeOf, flowNodesOf, newEdge, newNode, withFields } from './plans';
import {
  ANSWER_FIRST,
  ANSWER_FIRST_DETAIL,
  SAVE_FAILED,
  answerFirst,
  createSync,
  noteRename,
} from './plans-sync';
import type { Leave, PlansStore } from './plans-store.types';
import { useAppStore } from './use-app-store';

export { SAVE_DELAY } from './plans-sync';
export type { Leave } from './plans-store.types';

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });
/** A dialog asks about the failed save, so a notice saying the same goes. */
const dropSaveNotice = () => {
  const app = useAppStore.getState();
  if (app.notice?.title === SAVE_FAILED) app.dismissNotice();
  return false;
};

export const usePlansStore = create<PlansStore>((set, get) => {
  const sync = createSync(set, get);
  const { changed } = sync;
  const flush = async () => (await sync.flush()) || dropSaveNotice();
  const shut = () => {
    sync.reset(null);
    set({ open: null, nodes: [], edges: [], images: {}, dirty: false, saveError: null });
  };
  /** Leaves the open canvas for `leave` once it is saved; false and asks when it is not. */
  const settle = async (leave: Leave) => {
    if (!get().open) return true;
    // The outside change's dialog is up: leaving waits for its answer.
    if (get().outside) return answerFirst();
    if (await flush()) return true;
    set({ leaving: leave });
    return false;
  };
  const go = async (leave: Leave) => {
    if (leave.to === 'list') await get().close();
    else if (leave.to === 'canvas') await get().openCanvas(leave.id);
    else await get().loadList(leave.project);
  };
  const take = (doc: PlanDoc) => {
    sync.reset(doc.plan);
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
  };
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
    outside: null,
    history: EMPTY_HISTORY,
    arrived: [],
    marks: {},
    clearFresh: () => set({ fresh: null }),
    setArrived: arrived => set({ arrived }),
    clearMarks: () => set({ marks: {} }),
    ...sync.actions,

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

    create: async (name, preset = 'brainstorm') => {
      const project = get().project;
      if (!project || !(await settle({ to: 'list' }))) return null;
      try {
        const doc = await commands.planCreate(project, name, preset);
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
      if (get().outside) return answerFirst();
      await get().save();
      try {
        const summary = await commands.planRename(project, id, name);
        putSummary(summary);
        noteRename();
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

    saveForQuit: async () => {
      if (get().outside) return `${ANSWER_FIRST}. ${ANSWER_FIRST_DETAIL}`;
      return (await flush()) ? null : (get().saveError ?? SAVE_FAILED);
    },

    onNodesChange: changes => {
      const { nodes, edges } = get();
      const removed = changes.flatMap(c =>
        c.type === 'remove' && nodes.some(n => n.id === c.id && n.data.node.stop) ? [c.id] : []
      );
      set({ nodes: applyNodeChanges(changes, nodes) });
      // A stop taken out of a line joins the stops before and after it.
      if (removed.length) {
        const left = edges.filter(e => !removed.includes(e.source) && !removed.includes(e.target));
        set({ edges: healed(edges, left, removed, clock.nowMs()) });
      }
      // A drag records one undo step when it ends.
      changed(!changes.some(c => c.type === 'position' && c.dragging));
    },

    onEdgesChange: changes => {
      set({ edges: applyEdgeChanges(changes, get().edges) });
      changed();
    },

    onConnect: connection => {
      const stop = (id: string | null) => get().nodes.some(n => n.id === id && n.data.node.stop);
      // One stop to another leads to it: the tour order.
      const leads = stop(connection.source) && stop(connection.target);
      const edge = newEdge(connection, clock.nowMs(), leads ? 'leads-to' : 'relates');
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

    replaceFlow: (nodes, edges) => {
      set({ nodes, edges });
      changed();
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
        const id = get().addNode('image', at, { image: picked.path, title: picked.name });
        set({ arrived: [id] });
        return true;
      } catch (error) {
        notifyFailed('The picture was not added', error);
        return false;
      }
    },

    openLink: async node => {
      const { project, open } = get();
      if (!project || !open) return;
      // The link opens either way; the edits save once the outside change has its answer.
      if (!get().outside) await get().save();
      try {
        await commands.planOpenLink(project, open.id, node);
      } catch (error) {
        notifyFailed('The link did not open', error);
      }
    },
  };
});
