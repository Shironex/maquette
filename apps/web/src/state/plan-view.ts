/**
 * Where an open canvas keeps its nodes in sight: the part of the canvas no
 * floating control covers (the add bar on top, the zoom controls on the
 * left, the minimap at the bottom right while it shows), how the first fit
 * and Fit leave room for those controls, and panning to nodes that just
 * came. A narrow canvas folds the minimap into a Map button, so the bottom
 * gives back the room the map took.
 */
import type { RefObject } from 'react';
import type { ReactFlowInstance } from '@xyflow/react';
import type { FlowEdge, FlowNode } from './plans';

type Flow = ReactFlowInstance<FlowNode, FlowEdge>;

/** Insets from each edge of the canvas, in pixels. */
export interface Inset {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/** Below this canvas width, in pixels, the minimap folds into a Map button. */
export const MAP_FOLD_WIDTH = 960;

/** The part no control covers, with the minimap shown. */
export const SAFE_INSET: Inset = { top: 64, right: 32, bottom: 150, left: 60 };

/** The part no control covers with the minimap folded: the zoom controls sit low on the left. */
export const FOLDED_INSET: Inset = { ...SAFE_INSET, bottom: 64 };

/** The safe part's insets with the minimap shown or folded. */
export const insetFor = (map: boolean): Inset => (map ? SAFE_INSET : FOLDED_INSET);

/**
 * Whether a canvas this wide shows its minimap: the person's own choice
 * when they made one, else shown unless the canvas is narrow. A width of 0
 * (not laid out yet) counts as wide, so the first fit leaves the map room.
 */
export function mapShown(width: number, choice: boolean | null): boolean {
  if (choice !== null) return choice;
  return width === 0 || width >= MAP_FOLD_WIDTH;
}

/** The padding a fit leaves so no control covers a node, a little more than the safe part. */
export function fitPadding(map: boolean) {
  const inset = insetFor(map);
  const px = (n: number) => `${n + 8}px` as const;
  return {
    top: px(inset.top),
    right: px(inset.right),
    bottom: px(inset.bottom),
    left: px(inset.left),
  };
}

/** The safe part of the canvas on screen, or null before it is laid out. */
export function safeBox(frame: RefObject<HTMLElement | null>, inset: Inset = SAFE_INSET) {
  const box = frame.current?.getBoundingClientRect();
  if (!box || box.width === 0) return null;
  return {
    left: box.left + inset.left,
    top: box.top + inset.top,
    right: box.right - inset.right,
    bottom: box.bottom - inset.bottom,
  };
}

/** The smallest control in a node (the small icon button), in pixels at zoom 1. */
const SMALLEST_CONTROL_PX = 26;

/** The lowest zoom that keeps every control in a node at the 24px target size. */
export const READABLE_ZOOM = 24 / SMALLEST_CONTROL_PX;

type Bounds = ReturnType<Flow['getNodesBounds']>;

/** Centres `bounds` in the safe part at `zoom`. */
function centre(flow: Flow, bounds: Bounds, zoom: number, inset: Inset) {
  // The safe box's middle sits off the canvas's middle by half the insets' difference.
  const shift = {
    x: (inset.left - inset.right) / 2 / zoom,
    y: (inset.top - inset.bottom) / 2 / zoom,
  };
  void flow.setCenter(
    bounds.x + bounds.width / 2 - shift.x,
    bounds.y + bounds.height / 2 - shift.y,
    { zoom, duration: 250 }
  );
}

/**
 * Pans (and zooms out if it must) so the nodes `ids` sit inside the safe
 * part. With `readable` it zooms out no further than {@link READABLE_ZOOM};
 * nodes too spread for that bring the first of them into view instead.
 * `map` says whether the minimap shows, so the bottom keeps its room.
 */
export function reveal(
  flow: Flow,
  frame: RefObject<HTMLElement | null>,
  ids: string[],
  readable = false,
  map = true
) {
  const inset = insetFor(map);
  const safe = safeBox(frame, inset);
  const nodes = ids.filter(id => flow.getNode(id));
  if (!safe || nodes.length === 0) return;
  const bounds = flow.getNodesBounds(nodes);
  const a = flow.flowToScreenPosition({ x: bounds.x, y: bounds.y });
  const b = flow.flowToScreenPosition({ x: bounds.x + bounds.width, y: bounds.y + bounds.height });
  if (a.x >= safe.left && a.y >= safe.top && b.x <= safe.right && b.y <= safe.bottom) return;
  const zoom = flow.getZoom();
  const width = safe.right - safe.left;
  const height = safe.bottom - safe.top;
  if (b.x - a.x <= width && b.y - a.y <= height) {
    centre(flow, bounds, zoom, inset);
    return;
  }
  if (!readable) {
    void flow.fitView({
      nodes: nodes.map(id => ({ id })),
      padding: fitPadding(map),
      maxZoom: zoom,
      duration: 250,
    });
    return;
  }
  const fitting = Math.min(width / bounds.width, height / bounds.height, zoom);
  if (fitting >= READABLE_ZOOM) centre(flow, bounds, fitting, inset);
  else centre(flow, flow.getNodesBounds(nodes.slice(0, 1)), zoom, inset);
}
