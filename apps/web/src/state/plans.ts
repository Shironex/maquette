/**
 * The Plan view's rules: turning a canvas file into React Flow nodes and
 * edges and back, making new nodes and links, and sorting the list. The
 * file keeps whole pixels and the core sorts it, so a canvas moved and put
 * back writes the same file.
 */
import type { Connection, Edge, Node, XYPosition } from '@xyflow/react';
import type { NodeKind, PlanEdge, PlanFile, PlanNode, PlanSummary, Side } from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';

export type PlanNodeData = { node: PlanNode };
export type FlowNode = Node<PlanNodeData, NodeKind>;
export type FlowEdge = Edge<{ edge: PlanEdge }>;

/** The core's limits (crates/maquette-model/src/plans.rs), kept here so normal use never meets them. */
export const LIMITS = { title: 300, text: 20_000, coord: 1_000_000 } as const;

/** A position in whole pixels inside the range the file allows. */
export const coord = (value: number) =>
  Math.min(LIMITS.coord, Math.max(-LIMITS.coord, Math.round(value)));

/** How wide each kind draws when the file keeps no width. */
export const DEFAULT_WIDTH: Record<NodeKind, number> = { note: 220, image: 240, link: 260 };

/** The sides a node can be linked on; a handle's id is its side. */
export const SIDES: readonly Side[] = ['top', 'right', 'bottom', 'left'];

const isSide = (value: string | null | undefined): value is Side => SIDES.includes(value as Side);

/** A short id that sorts by when it was made: `n-` or `e-`, the time, a little noise. */
export function newId(prefix: 'n' | 'e', nowMs: number, random = Math.random): string {
  const noise = Math.floor(random() * 36 ** 3)
    .toString(36)
    .padStart(3, '0');
  return `${prefix}-${nowMs.toString(36)}${noise}`;
}

const KIND_NAME: Record<NodeKind, string> = { note: 'Note', image: 'Picture', link: 'Link' };

/** What a screen reader hears for a node: its kind and title. */
export function nodeLabel(node: PlanNode): string {
  return `${KIND_NAME[node.kind]}: ${nodeTitle(node)}`;
}

export function flowNodeOf(node: PlanNode): FlowNode {
  return {
    id: node.id,
    type: node.kind,
    position: { x: node.x, y: node.y },
    width: node.width ?? DEFAULT_WIDTH[node.kind],
    ariaLabel: nodeLabel(node),
    data: { node },
  };
}

export function flowNodesOf(plan: PlanFile): FlowNode[] {
  return plan.nodes.map(flowNodeOf);
}

/** How tall a node is taken to be before it has drawn once. */
const GUESS_HEIGHT: Record<NodeKind, number> = { note: 110, image: 250, link: 130 };
const GAP = 24;

/**
 * Where a new node of `kind` goes near `want` without covering another:
 * `want` itself when it is free, else the nearest free spot on rings of
 * steps around it, else `want`.
 */
export function freeSpot(nodes: readonly FlowNode[], want: XYPosition, kind: NodeKind): XYPosition {
  const w = DEFAULT_WIDTH[kind];
  const h = GUESS_HEIGHT[kind];
  const boxes = nodes.map(n => ({
    x: n.position.x,
    y: n.position.y,
    w: n.measured?.width ?? n.width ?? DEFAULT_WIDTH[n.data.node.kind],
    h: n.measured?.height ?? n.height ?? GUESS_HEIGHT[n.data.node.kind],
  }));
  const free = (x: number, y: number) =>
    boxes.every(
      b => x + w + GAP <= b.x || b.x + b.w + GAP <= x || y + h + GAP <= b.y || b.y + b.h + GAP <= y
    );
  const step = 60;
  for (let ring = 0; ring <= 12; ring += 1) {
    const spots: XYPosition[] = [];
    for (let i = -ring; i <= ring; i += 1) {
      spots.push({ x: i, y: -ring }, { x: i, y: ring }, { x: -ring, y: i }, { x: ring, y: i });
    }
    const found = spots
      .map(s => ({ x: Math.round(want.x + s.x * step), y: Math.round(want.y + s.y * step) }))
      .filter(s => free(s.x, s.y))
      .sort(
        (a, b) => Math.hypot(a.x - want.x, a.y - want.y) - Math.hypot(b.x - want.x, b.y - want.y)
      );
    if (found[0]) return found[0];
  }
  return want;
}

export function flowEdgeOf(edge: PlanEdge): FlowEdge {
  return {
    id: edge.id,
    source: edge.from,
    target: edge.to,
    sourceHandle: edge.fromSide,
    targetHandle: edge.toSide,
    label: edge.label ?? undefined,
    data: { edge },
  };
}

export function flowEdgesOf(plan: PlanFile): FlowEdge[] {
  return plan.edges.map(flowEdgeOf);
}

/** The canvas file the flow shows, positions in whole pixels. */
export function planOf(base: PlanFile, nodes: FlowNode[], edges: FlowEdge[]): PlanFile {
  return {
    ...base,
    nodes: nodes.map(n => ({
      ...n.data.node,
      x: coord(n.position.x),
      y: coord(n.position.y),
    })),
    edges: edges.map(e => ({
      ...(e.data?.edge ?? {
        id: e.id,
        kind: 'relates' as const,
        label: null,
        by: null,
      }),
      from: e.source,
      to: e.target,
      fromSide: isSide(e.sourceHandle) ? e.sourceHandle : null,
      toSide: isSide(e.targetHandle) ? e.targetHandle : null,
    })),
  };
}

const sorted = (plan: PlanFile) => ({
  name: plan.name,
  nodes: [...plan.nodes].sort((a, b) => a.id.localeCompare(b.id)),
  edges: [...plan.edges].sort((a, b) => a.id.localeCompare(b.id)),
});

/** True when two canvases would write the same file. */
export function samePlan(a: PlanFile, b: PlanFile): boolean {
  return deepEqual(sorted(a), sorted(b));
}

export interface NodeFields {
  title?: string;
  text?: string;
  image?: string | null;
  url?: string | null;
}

/** A new node of `kind` with its top left corner at `at`. */
export function newNode(
  kind: NodeKind,
  at: { x: number; y: number },
  fields: NodeFields,
  nowMs: number
): PlanNode {
  return {
    id: newId('n', nowMs),
    kind,
    x: coord(at.x),
    y: coord(at.y),
    width: null,
    title: (fields.title ?? '').slice(0, LIMITS.title),
    text: (fields.text ?? '').slice(0, LIMITS.text),
    image: fields.image ?? null,
    url: fields.url ?? null,
    by: null,
  };
}

/** A link between two nodes as a person draws it: they relate. */
export function newEdge(connection: Connection, nowMs: number): FlowEdge | null {
  const { source, target } = connection;
  if (!source || !target || source === target) return null;
  const edge: PlanEdge = {
    id: newId('e', nowMs),
    from: source,
    to: target,
    fromSide: isSide(connection.sourceHandle) ? connection.sourceHandle : null,
    toSide: isSide(connection.targetHandle) ? connection.targetHandle : null,
    kind: 'relates',
    label: null,
    by: null,
  };
  return flowEdgeOf(edge);
}

/**
 * The address a person typed, as the file keeps it: `https://` added when
 * it has no scheme, `null` when it is not an http or https address.
 */
export function webAddress(typed: string): string | null {
  const text = typed.trim();
  if (!text || /\s/.test(text)) return null;
  const withScheme = /^[a-z][a-z0-9+.-]*:/i.test(text) ? text : `https://${text}`;
  try {
    const url = new URL(withScheme);
    if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
    return url.hostname ? withScheme : null;
  } catch {
    return null;
  }
}

/** The host of a link, for its footer: `example.com`. */
export function hostOf(url: string | null): string {
  if (!url) return '';
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

/** A node's one line name, for the minimap, search and dialogs. */
export function nodeTitle(node: PlanNode): string {
  const title = node.title.trim();
  if (title) return title;
  const first = node.text.trim().split('\n')[0] ?? '';
  if (first) return first.length > 60 ? `${first.slice(0, 59)}...` : first;
  return node.kind === 'note' ? 'Empty note' : node.kind === 'image' ? 'Picture' : 'Link';
}

/** The canvases edited last first, then by name. */
export function byEdited(plans: readonly PlanSummary[]): PlanSummary[] {
  return [...plans].sort(
    (a, b) => b.edited.localeCompare(a.edited) || a.name.localeCompare(b.name)
  );
}

/** The canvases whose name or file holds `query`, ignoring case. */
export function findPlans(plans: readonly PlanSummary[], query: string): PlanSummary[] {
  const q = query.trim().toLowerCase();
  if (!q) return [...plans];
  return plans.filter(p => p.name.toLowerCase().includes(q) || p.file.toLowerCase().includes(q));
}

/** "9 nodes · 3 pictures · 2 links", leaving out what a canvas has none of. */
export function contentLine(plan: PlanSummary): string {
  const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  const parts = [count(plan.nodes, 'node', 'nodes')];
  if (plan.images) parts.push(count(plan.images, 'picture', 'pictures'));
  if (plan.links) parts.push(count(plan.links, 'link', 'links'));
  return parts.join(' · ');
}
