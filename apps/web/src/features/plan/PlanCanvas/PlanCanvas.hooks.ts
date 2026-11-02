import { useEffect, useRef, useState, type MouseEvent } from 'react';
import { useReactFlow, useViewport } from '@xyflow/react';
import type { NodeKind } from '@/ipc/client';
import { typing } from '@/lib/typing';
import { DEFAULT_WIDTH, freeSpot } from '@/state/plans';
import { usePlansStore } from '@/state/use-plans-store';

/** Keys that add a node while the canvas is open. */
export const ADD_KEYS: Record<string, NodeKind> = { n: 'note', i: 'image', l: 'link' };

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
  const flow = useReactFlow();
  const { zoom } = useViewport();
  const frame = useRef<HTMLDivElement>(null);
  const [linking, setLinking] = useState(false);

  const middle = (kind: NodeKind) => {
    const box = frame.current?.getBoundingClientRect();
    const centre = box
      ? flow.screenToFlowPosition({ x: box.left + box.width / 2, y: box.top + box.height / 2 })
      : { x: 0, y: 0 };
    const want = { x: centre.x - DEFAULT_WIDTH[kind] / 2, y: centre.y - 60 };
    return freeSpot(usePlansStore.getState().nodes, want, kind);
  };
  const add = (kind: NodeKind) => {
    if (kind === 'note') addNode('note', middle('note'));
    else if (kind === 'image') void addPicture(middle('image'));
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
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    frame,
    add,
    linking,
    closeLink: () => setLinking(false),
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
    zoomPercent: `${Math.round(zoom * 100)}%`,
    zoomIn: () => void flow.zoomIn({ duration: 150 }),
    zoomOut: () => void flow.zoomOut({ duration: 150 }),
    fit: () => void flow.fitView({ duration: 200, padding: 0.2 }),
    back: () => void close(),
    saveLine: saving ? 'saving' : saveError ? 'not saved, edit again to retry' : 'saved',
    saveFailed: Boolean(saveError) && !saving,
  };
}
