import { useEffect, useMemo, useRef, useState, type MouseEvent } from 'react';
import { useReactFlow, useViewport } from '@xyflow/react';
import type { NodeKind } from '@/ipc/client';
import { typing } from '@/lib/typing';
import { anyRunning, assetView, directionView, linkedIds, type LinkPick } from '@/state/plan-links';
import { DEFAULT_WIDTH, freeSpot, isLinked, nodeLabel, type FlowNode } from '@/state/plans';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import {
  CHANGE_REREAD_MS,
  RUNNING_REREAD_MS,
  usePlanLinksStore,
} from '@/state/use-plan-links-store';
import { usePlansStore } from '@/state/use-plans-store';

/** What the bar adds; an asset or Lab direction comes from the Asset picker. */
export type AddKind = 'note' | 'image' | 'link' | 'asset';

/** Keys that add a node while the canvas is open. */
export const ADD_KEYS: Record<string, AddKind> = { n: 'note', i: 'image', l: 'link', a: 'asset' };

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
  const flow = useReactFlow();
  const { zoom } = useViewport();
  const frame = useRef<HTMLDivElement>(null);
  const [linking, setLinking] = useState(false);
  const [picking, setPicking] = useState(false);
  const named = useNamedNodes(nodes);
  useLiveLinks();

  const middle = (kind: NodeKind) => {
    const box = frame.current?.getBoundingClientRect();
    const centre = box
      ? flow.screenToFlowPosition({ x: box.left + box.width / 2, y: box.top + box.height / 2 })
      : { x: 0, y: 0 };
    const want = { x: centre.x - DEFAULT_WIDTH[kind] / 2, y: centre.y - 60 };
    return freeSpot(usePlansStore.getState().nodes, want, kind);
  };
  const add = (kind: AddKind) => {
    if (kind === 'note') addNode('note', middle('note'));
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
        for (const { kind, id } of picked) addNode(kind, middle(kind), { [kind]: id });
      },
    },
    addLink: (url: string, title: string) => {
      setLinking(false);
      addNode('link', middle('link'), { url, title });
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
