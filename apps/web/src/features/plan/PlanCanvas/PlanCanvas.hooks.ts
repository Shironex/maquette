import { useEffect, useMemo, useRef, useState, type MouseEvent, type RefObject } from 'react';
import { useReactFlow, useViewport, type ReactFlowInstance } from '@xyflow/react';
import type { NodeKind } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { typing } from '@/lib/typing';
import { anyRunning, assetView, directionView, linkedIds, type LinkPick } from '@/state/plan-links';
import { unseenCount, whenOf } from '@/state/plan-log';
import {
  DEFAULT_WIDTH,
  freeSpot,
  isLinked,
  nodeLabel,
  type FlowEdge,
  type FlowNode,
} from '@/state/plans';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import {
  CHANGE_REREAD_MS,
  RUNNING_REREAD_MS,
  usePlanLinksStore,
} from '@/state/use-plan-links-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';

/** What the bar adds; an asset or Lab direction comes from the Asset picker. */
export type AddKind = 'note' | 'image' | 'link' | 'asset';

/** Keys that add a node while the canvas is open. */
export const ADD_KEYS: Record<string, AddKind> = { n: 'note', i: 'image', l: 'link', a: 'asset' };

/**
 * The part of the canvas no floating control covers, in pixels from each
 * edge: the add bar on top, the zoom controls on the left, the minimap at
 * the bottom right. New nodes land in it, and the view pans to keep the
 * ones that just came inside it.
 */
export const SAFE_INSET = { top: 64, right: 32, bottom: 150, left: 60 };

type Flow = ReactFlowInstance<FlowNode, FlowEdge>;

/** The safe part of the canvas on screen, or null before it is laid out. */
function safeBox(frame: RefObject<HTMLElement | null>) {
  const box = frame.current?.getBoundingClientRect();
  if (!box || box.width === 0) return null;
  return {
    left: box.left + SAFE_INSET.left,
    top: box.top + SAFE_INSET.top,
    right: box.right - SAFE_INSET.right,
    bottom: box.bottom - SAFE_INSET.bottom,
  };
}

/** Pans (and zooms out only if it must) so the nodes `ids` sit inside the safe part. */
function reveal(flow: Flow, frame: RefObject<HTMLElement | null>, ids: string[]) {
  const safe = safeBox(frame);
  const nodes = ids.filter(id => flow.getNode(id));
  if (!safe || nodes.length === 0) return;
  const bounds = flow.getNodesBounds(nodes);
  const a = flow.flowToScreenPosition({ x: bounds.x, y: bounds.y });
  const b = flow.flowToScreenPosition({ x: bounds.x + bounds.width, y: bounds.y + bounds.height });
  if (a.x >= safe.left && a.y >= safe.top && b.x <= safe.right && b.y <= safe.bottom) return;
  const zoom = flow.getZoom();
  const fits = b.x - a.x <= safe.right - safe.left && b.y - a.y <= safe.bottom - safe.top;
  if (!fits) {
    void flow.fitView({
      nodes: nodes.map(id => ({ id })),
      padding: 0.3,
      maxZoom: zoom,
      duration: 250,
    });
    return;
  }
  // The safe box's middle sits off the canvas's middle by half the insets' difference.
  const shift = {
    x: (SAFE_INSET.left - SAFE_INSET.right) / 2 / zoom,
    y: (SAFE_INSET.top - SAFE_INSET.bottom) / 2 / zoom,
  };
  void flow.setCenter(
    bounds.x + bounds.width / 2 - shift.x,
    bounds.y + bounds.height / 2 - shift.y,
    { zoom, duration: 250 }
  );
}

/**
 * The canvas's history: Undo and Redo of the person's edits (buttons and
 * Mod+Z, Shift+Mod+Z, Mod+Y), the Changes panel's button and count, the
 * chip naming the agent that wrote last, the choice when an outside edit
 * meets unsaved edits to the same nodes, and bringing nodes that just came
 * into sight.
 */
export function useCanvasHistory(flow: Flow, frame: RefObject<HTMLElement | null>) {
  const history = usePlansStore(s => s.history);
  const undo = usePlansStore(s => s.undo);
  const redo = usePlansStore(s => s.redo);
  const outside = usePlansStore(s => s.outside);
  const keepMine = usePlansStore(s => s.keepMine);
  const useTheirs = usePlansStore(s => s.useTheirs);
  const arrived = usePlansStore(s => s.arrived);
  const setArrived = usePlansStore(s => s.setArrived);
  const lines = usePlanLogStore(s => s.lines);
  const panel = usePlanLogStore(s => s.panel);
  const seenAt = usePlanLogStore(s => s.seenAt);
  const setPanel = usePlanLogStore(s => s.setPanel);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey || typing(e.target)) return;
      if (document.querySelector('[aria-modal="true"]')) return;
      const key = e.key.toLowerCase();
      const s = usePlansStore.getState();
      if (key === 'z') {
        e.preventDefault();
        if (e.shiftKey) s.redo();
        else s.undo();
      } else if (key === 'y') {
        e.preventDefault();
        s.redo();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // Waits a frame so the new nodes are measured before the view moves to them.
  useEffect(() => {
    if (arrived.length === 0) return;
    const timer = setTimeout(() => {
      reveal(flow, frame, arrived);
      setArrived([]);
    }, 60);
    return () => clearTimeout(timer);
  }, [arrived, flow, frame, setArrived]);

  // The Changes panel narrows the canvas: the nodes an outside write marked move back clear of its edge.
  useEffect(() => {
    if (!panel) return;
    const timer = setTimeout(
      () => reveal(flow, frame, Object.keys(usePlansStore.getState().marks)),
      60
    );
    return () => clearTimeout(timer);
  }, [panel, flow, frame]);

  const last = lines[0];
  const agent = last && last.by !== null ? last : null;
  return {
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    undo,
    redo,
    changes: {
      open: panel,
      unseen: panel ? 0 : unseenCount(lines, seenAt),
      toggle: () => setPanel(!panel),
    },
    /** The agent that wrote last, for the chip: "Research agent · 12:41". */
    agent: agent ? { name: agent.by ?? '', when: whenOf(agent.at, clock.now()) } : null,
    outside: outside
      ? { count: outside.conflicts.length, keepMine, useTheirs: () => useTheirs() }
      : null,
  };
}

/**
 * The live read of what the canvas links: again when it opens, when its
 * links change, when the window comes back, every few seconds while a
 * linked Lab round is being made, and once things settle after the
 * project or the Inbox changes (they change on every job event, so those
 * reads wait `CHANGE_REREAD_MS` for the last one). Leaving the canvas
 * forgets the read.
 */
export function useLiveLinks() {
  const project = usePlansStore(s => s.project);
  const openId = usePlansStore(s => s.open?.id ?? null);
  const nodes = usePlansStore(s => s.nodes);
  const manifest = useAppStore(s => s.workspace?.project ?? null);
  const inbox = useInboxStore(s => s.snapshot);
  const refresh = usePlanLinksStore(s => s.refresh);
  const clear = usePlanLinksStore(s => s.clear);
  const running = usePlanLinksStore(s => anyRunning(s.directions));
  const key = JSON.stringify(linkedIds(nodes.map(n => n.data.node)));
  const read = useRef<(() => void) | null>(null);
  useEffect(() => {
    if (!project || !openId) return;
    const now = () => void refresh(project, JSON.parse(key) as ReturnType<typeof linkedIds>);
    read.current = now;
    now();
    window.addEventListener('focus', now);
    const timer = running ? setInterval(now, RUNNING_REREAD_MS) : undefined;
    return () => {
      read.current = null;
      window.removeEventListener('focus', now);
      clearInterval(timer);
    };
  }, [project, openId, key, running, refresh]);
  const seen = useRef({ manifest, inbox });
  useEffect(() => {
    if (seen.current.manifest === manifest && seen.current.inbox === inbox) return;
    seen.current = { manifest, inbox };
    const timer = setTimeout(() => read.current?.(), CHANGE_REREAD_MS);
    return () => clearTimeout(timer);
  }, [manifest, inbox]);
  useEffect(() => clear, [clear]);
}

/** The flow's nodes with an asset or direction node named by its live read, for screen readers. */
export function useNamedNodes(nodes: FlowNode[]): FlowNode[] {
  const assets = usePlanLinksStore(s => s.assets);
  const directions = usePlanLinksStore(s => s.directions);
  return useMemo(
    () =>
      nodes.map(n => {
        const node = n.data.node;
        if (!isLinked(node.kind)) return n;
        const view =
          node.kind === 'asset'
            ? assetView(assets[node.asset ?? ''])
            : directionView(directions[node.direction ?? '']);
        return { ...n, ariaLabel: nodeLabel(node, view.name) };
      }),
    [nodes, assets, directions]
  );
}

/**
 * The open canvas: its nodes and edges from the store, where a new node
 * lands (near the middle of the view, clear of other nodes), the add
 * keys, the zoom controls and the save line.
 */
export function usePlanCanvas() {
  const open = usePlansStore(s => s.open);
  const nodes = usePlansStore(s => s.nodes);
  const edges = usePlansStore(s => s.edges);
  const saving = usePlansStore(s => s.saving);
  const saveError = usePlansStore(s => s.saveError);
  const onNodesChange = usePlansStore(s => s.onNodesChange);
  const onEdgesChange = usePlansStore(s => s.onEdgesChange);
  const onConnect = usePlansStore(s => s.onConnect);
  const addNode = usePlansStore(s => s.addNode);
  const addPicture = usePlansStore(s => s.addPicture);
  const close = usePlansStore(s => s.close);
  const leaving = usePlansStore(s => s.leaving);
  const retryLeave = usePlansStore(s => s.retryLeave);
  const leaveAnyway = usePlansStore(s => s.leaveAnyway);
  const stay = usePlansStore(s => s.stay);
  const setArrived = usePlansStore(s => s.setArrived);
  const flow = useReactFlow<FlowNode, FlowEdge>();
  const { zoom } = useViewport();
  const frame = useRef<HTMLDivElement>(null);
  const [linking, setLinking] = useState(false);
  const [picking, setPicking] = useState(false);
  const named = useNamedNodes(nodes);
  useLiveLinks();
  const history = useCanvasHistory(flow, frame);

  /** Near the middle of the part of the view no control covers. */
  const middle = (kind: NodeKind) => {
    const safe = safeBox(frame);
    const centre = safe
      ? flow.screenToFlowPosition({
          x: (safe.left + safe.right) / 2,
          y: (safe.top + safe.bottom) / 2,
        })
      : { x: 0, y: 0 };
    const want = { x: centre.x - DEFAULT_WIDTH[kind] / 2, y: centre.y - 60 };
    return freeSpot(usePlansStore.getState().nodes, want, kind);
  };
  const add = (kind: AddKind) => {
    if (kind === 'note') setArrived([addNode('note', middle('note'))]);
    else if (kind === 'image') void addPicture(middle('image'));
    else if (kind === 'asset') setPicking(true);
    else setLinking(true);
  };
  const addRef = useRef(add);
  useEffect(() => {
    addRef.current = add;
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return;
      if (document.querySelector('[aria-modal="true"]')) return;
      const kind = ADD_KEYS[e.key.toLowerCase()];
      if (!kind) return;
      e.preventDefault();
      addRef.current(kind);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return {
    open,
    history,
    nodes: named,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    frame,
    add,
    linking,
    closeLink: () => setLinking(false),
    /** The Asset picker: what the canvas links already, and adding what was picked. */
    picker: {
      open: picking,
      linked: linkedIds(nodes.map(n => n.data.node)),
      close: () => setPicking(false),
      /** Each picked asset or direction lands near the middle, clear of the others. */
      add: (picked: LinkPick[]) => {
        setPicking(false);
        const ids = picked.map(({ kind, id }) => addNode(kind, middle(kind), { [kind]: id }));
        setArrived(ids);
      },
    },
    addLink: (url: string, title: string) => {
      setLinking(false);
      setArrived([addNode('link', middle('link'), { url, title })]);
    },
    /** A double click on the empty canvas writes a note there. */
    onDoubleClick: (e: MouseEvent) => {
      if (!(e.target instanceof Element) || !e.target.classList.contains('react-flow__pane'))
        return;
      const at = flow.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      addNode('note', { x: at.x - 20, y: at.y - 20 });
    },
    zoom: {
      percent: `${Math.round(zoom * 100)}%`,
      in: () => void flow.zoomIn({ duration: 150 }),
      out: () => void flow.zoomOut({ duration: 150 }),
      fit: () => void flow.fitView({ duration: 200, padding: 0.2 }),
    },
    back: () => void close(),
    saveLine: saving ? 'saving' : saveError ? 'not saved, edit again to retry' : 'saved',
    saveFailed: Boolean(saveError) && !saving,
    /** A save failed on the way out: the question and its answers. */
    leave: {
      asking: leaving !== null,
      error: saveError,
      retry: () => void retryLeave(),
      anyway: () => void leaveAnyway(),
      stay,
    },
  };
}
