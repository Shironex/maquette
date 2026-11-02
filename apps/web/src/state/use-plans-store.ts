/**
 * The Plan view's state: the project's canvases, the one open and its
 * nodes and edges as React Flow holds them. Every change that alters the
 * file saves it half a second later; selecting and measuring do not.
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
  nodeLabel,
  planOf,
  samePlan,
  type FlowEdge,
  type FlowNode,
  type NodeFields,
} from './plans';
import { useAppStore } from './use-app-store';

/** How long after the last change the canvas saves, in milliseconds. */
export const SAVE_DELAY = 500;

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
  /** A note just made, which opens for writing. */
  fresh: string | null;
  clearFresh: () => void;
  loadList: (project: string) => Promise<void>;
  /** Makes a canvas and opens it; its id, or null when refused. */
  create: (name: string) => Promise<string | null>;
  rename: (id: string, name: string) => Promise<boolean>;
  remove: (id: string) => Promise<boolean>;
  openCanvas: (id: string) => Promise<void>;
  /** Back to the list, saving first. */
  close: () => Promise<void>;
  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange<FlowEdge>[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (kind: NodeKind, at: { x: number; y: number }, fields?: NodeFields) => string;
  updateNode: (id: string, fields: NodeFields) => void;
  /** Picks a picture and adds it at `at`; false when cancelled or refused. */
  addPicture: (at: { x: number; y: number }) => Promise<boolean>;
  openLink: (node: string) => Promise<void>;
  /** Saves the open canvas now, when it changed. */
  save: () => Promise<boolean>;
}

let timer: ReturnType<typeof setTimeout> | undefined;

function withFields(flow: FlowNode, fields: NodeFields): FlowNode {
  const node = { ...flow.data.node, ...fields };
  return { ...flow, ariaLabel: nodeLabel(node), data: { node } };
}

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

export const usePlansStore = create<PlansStore>((set, get) => {
  const draft = () => {
    const { open, nodes, edges } = get();
    return open ? planOf(open.plan, nodes, edges) : null;
  };
  const changed = () => {
    const next = draft();
    const open = get().open;
    if (!next || !open || samePlan(next, open.plan)) return;
    clearTimeout(timer);
    timer = setTimeout(() => void get().save(), SAVE_DELAY);
  };
  const take = (doc: PlanDoc) =>
    set({
      open: doc,
      opening: null,
      nodes: flowNodesOf(doc.plan),
      edges: flowEdgesOf(doc.plan),
      images: doc.images,
      saveError: null,
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
    fresh: null,
    clearFresh: () => set({ fresh: null }),

    loadList: async project => {
      if (get().project !== project) {
        clearTimeout(timer);
        set({ project, list: null, open: null, nodes: [], edges: [], images: {} });
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
      if (!project) return null;
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
        if (get().open?.id === id) {
          clearTimeout(timer);
          set({ open: null, nodes: [], edges: [], images: {} });
        }
        set(s => ({ list: s.list?.filter(p => p.id !== id) ?? null }));
        return true;
      } catch (error) {
        notifyFailed('The canvas was not removed', error);
        return false;
      }
    },

    openCanvas: async id => {
      const project = get().project;
      if (!project) return;
      await get().save();
      set({ opening: id, openError: null });
      try {
        const doc = await commands.planGet(project, id);
        if (get().opening === id) take(doc);
      } catch (error) {
        if (get().opening === id) set({ opening: null, openError: errorMessage(error) });
      }
    },

    close: async () => {
      await get().save();
      const project = get().project;
      set({ open: null, nodes: [], edges: [], images: {}, openError: null });
      if (project) await get().loadList(project);
    },

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

    save: async () => {
      clearTimeout(timer);
      const { project, open, saving } = get();
      const next = draft();
      if (!project || !open || !next || saving || samePlan(next, open.plan)) return true;
      set({ saving: true });
      try {
        const doc = await commands.planSave(project, open.id, next);
        if (get().open?.id !== open.id) {
          set({ saving: false });
          return true;
        }
        set({
          open: doc,
          images: { ...get().images, ...doc.images },
          saving: false,
          saveError: null,
        });
        // Changes made while it saved go next.
        changed();
        return true;
      } catch (error) {
        set({ saving: false, saveError: errorMessage(error) });
        notifyFailed('The canvas was not saved', error);
        return false;
      }
    },
  };
});
