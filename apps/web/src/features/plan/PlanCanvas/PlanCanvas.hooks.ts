import { useEffect, useMemo, useRef, useState, type MouseEvent, type RefObject } from 'react';
import { useReactFlow, useStore, useViewport, type ReactFlowInstance } from '@xyflow/react';
import type { NodeKind } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { typing } from '@/lib/typing';
import { CAMERA_PLAN_ID } from '@/state/plan-camera';
import { anyRunning, assetView, directionView, linkedIds, type LinkPick } from '@/state/plan-links';
import { unseenCount, whenOf } from '@/state/plan-log';
import { fitPadding, insetFor, MAP_FOLD_WIDTH, mapShown, reveal, safeBox } from '@/state/plan-view';
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
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';

/** What the bar adds; an asset or Lab direction comes from the Asset picker. */
export type AddKind = 'note' | 'image' | 'link' | 'asset';

/** Keys that add a node while the canvas is open. */
export const ADD_KEYS: Record<string, AddKind> = { n: 'note', i: 'image', l: 'link', a: 'asset' };

type Flow = ReactFlowInstance<FlowNode, FlowEdge>;

/**
 * The canvas's history: Undo and Redo of the person's edits (buttons and
 * Mod+Z, Shift+Mod+Z, Mod+Y), the Changes panel's button and count, the
 * chip naming the agent that wrote last, the choice when an outside edit
 * meets unsaved edits to the same nodes, and bringing nodes that just came
 * into sight.
 */
export function useCanvasHistory(flow: Flow, frame: RefObject<HTMLElement | null>, map: boolean) {
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
  // Escape or a click outside closes the clash question without choosing; saves stay blocked until it is answered.
  const [hidden, setHidden] = useState<object | null>(null);

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
      reveal(flow, frame, arrived, false, map);
      setArrived([]);
    }, 60);
    return () => clearTimeout(timer);
  }, [arrived, flow, frame, setArrived, map]);

  // The Changes panel narrows the canvas: the nodes an outside write marked move back clear of its edge.
  useEffect(() => {
    if (!panel) return;
    const timer = setTimeout(
      () => reveal(flow, frame, Object.keys(usePlansStore.getState().marks), true, map),
      60
    );
    return () => clearTimeout(timer);
  }, [panel, flow, frame, map]);

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
      toggle: () => {
        // One side panel at a time: Changes takes the Overlaps panel's place.
        if (!panel) usePlanExtrasStore.getState().setOverlaps(false);
        setPanel(!panel);
      },
    },
    /** The agent that wrote last, for the chip: "Research agent · 12:41". */
    agent: agent ? { name: agent.by ?? '', when: whenOf(agent.at, clock.now()) } : null,
    outside: outside
      ? {
          count: outside.conflicts.length,
          asking: hidden !== outside,
          keepMine,
          useTheirs: () => useTheirs(),
          later: () => setHidden(outside),
          ask: () => setHidden(null),
        }
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
  const clash = usePlansStore(s => Boolean(s.outside));
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
  const width = useStore(s => s.width);
  // Null until the person shows or hides the map; a narrow canvas folds it on its own.
  const [mapChoice, setMapChoice] = useState<boolean | null>(null);
  const map = mapShown(width, mapChoice);
  const frame = useRef<HTMLDivElement>(null);
  const [linking, setLinking] = useState(false);
  const [picking, setPicking] = useState(false);
  const [cameraPanel, setCameraPanel] = useState(true);
  const named = useNamedNodes(nodes);
  useLiveLinks();
  const history = useCanvasHistory(flow, frame, map);

  /** Near the middle of the part of the view no control covers. */
  const middle = (kind: NodeKind) => {
    const safe = safeBox(frame, insetFor(map));
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
    /** The camera plan, with its panel open or folded. */
    camera: open?.id === CAMERA_PLAN_ID ? { panel: cameraPanel, setPanel: setCameraPanel } : null,
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
      fit: () => void flow.fitView({ duration: 200, padding: fitPadding(map) }),
      /** The minimap, folded into a Map button on a narrow canvas or when the person hid it. */
      map: {
        shown: map,
        toggle: width > 0 && (width < MAP_FOLD_WIDTH || mapChoice !== null),
        flip: () => setMapChoice(!map),
        /** The first fit leaves room for the map and the controls. */
        fit: { padding: fitPadding(map), maxZoom: 1 },
      },
    },
    back: () => void close(),
    saveLine: clash
      ? 'not saved, answer the clash'
      : saving
        ? 'saving'
        : saveError
          ? 'not saved, edit again to retry'
          : 'saved',
    saveFailed: clash || (Boolean(saveError) && !saving),
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
