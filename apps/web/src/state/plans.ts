/**
 * The Plan view's rules: turning a canvas file into React Flow nodes and
 * edges and back, making new nodes and links, and sorting the list. The
 * file keeps whole pixels and the core sorts it, so a canvas moved and put
 * back writes the same file.
 */
import type { Connection, Edge, Node, XYPosition } from '@xyflow/react';
import type {
  EdgeKind,
  LayoutStop,
  NodeKind,
  PlanEdge,
  PlanFile,
  PlanNode,
  PlanSummary,
  Side,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { trimText } from './plan-text';

export type PlanNodeData = { node: PlanNode };
export type FlowNode = Node<PlanNodeData, NodeKind>;
export type FlowEdge = Edge<{ edge: PlanEdge }>;

/** The core's limits (crates/maquette-model/src/plans.rs), kept here so normal use never meets them. */
export const LIMITS = { title: 300, text: 20_000, coord: 1_000_000 } as const;

/** A position in whole pixels inside the range the file allows. */
export const coord = (value: number) =>
  Math.min(LIMITS.coord, Math.max(-LIMITS.coord, Math.round(value)));

/** How wide each kind draws when the file keeps no width. */
export const DEFAULT_WIDTH: Record<NodeKind, number> = {
  note: 220,
  image: 240,
  link: 260,
  asset: 248,
  direction: 220,
  stop: 220,
};

/** Kinds that link something of the project and read its name live. */
export const isLinked = (kind: NodeKind): kind is 'asset' | 'direction' =>
  kind === 'asset' || kind === 'direction';

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

const KIND_NAME: Record<NodeKind, string> = {
  note: 'Note',
  image: 'Picture',
  link: 'Link',
  asset: 'Asset',
  direction: 'Lab direction',
  stop: 'Camera stop',
};

/**
 * What a screen reader hears for a node: its kind and title. An asset or
 * direction node keeps no title, so its live `name` is passed in.
 */
export function nodeLabel(node: PlanNode, name?: string): string {
  return `${KIND_NAME[node.kind]}: ${name ?? nodeTitle(node)}`;
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
const GUESS_HEIGHT: Record<NodeKind, number> = {
  note: 110,
  image: 250,
  link: 130,
  asset: 120,
  direction: 190,
  stop: 230,
};
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
    labelBgPadding: [6, 3],
    labelBgBorderRadius: 4,
    data: { edge },
  };
}

export function flowEdgesOf(plan: PlanFile): FlowEdge[] {
  return plan.edges.map(flowEdgeOf);
}

/**
 * The flow for `plan` when the canvas changed under it (an outside write,
 * an undo): nodes and edges it had keep their selection and measured size.
 */
export function flowFrom(
  plan: PlanFile,
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[]
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const hadNode = new Map(nodes.map(n => [n.id, n]));
  const hadEdge = new Map(edges.map(e => [e.id, e]));
  return {
    nodes: plan.nodes.map(node => {
      const fresh = flowNodeOf(node);
      const had = hadNode.get(node.id);
      return had ? { ...fresh, selected: had.selected, measured: had.measured } : fresh;
    }),
    edges: plan.edges.map(edge => {
      const fresh = flowEdgeOf(edge);
      const had = hadEdge.get(edge.id);
      return had ? { ...fresh, selected: had.selected } : fresh;
    }),
  };
}

/** A node's fields as the core writes them: a trimmed title and address. */
export function normalNode(node: PlanNode): PlanNode {
  const url = node.url === null ? null : trimText(node.url);
  return { ...node, title: trimText(node.title), url };
}

/** An edge label as the core writes it: trimmed, and none when empty. */
const normalLabel = (label: string | null) => (label === null ? null : trimText(label) || null);

/**
 * The canvas file the flow shows, positions in whole pixels and text
 * normalised the way the core normalises a save (`normalized` in
 * crates/maquette-model/src/plans.rs), so a canvas the core trimmed reads
 * as saved and does not save again and again.
 */
export function planOf(base: PlanFile, nodes: FlowNode[], edges: FlowEdge[]): PlanFile {
  return {
    ...base,
    name: trimText(base.name),
    nodes: nodes.map(n => ({
      ...normalNode(n.data.node),
      x: coord(n.position.x),
      y: coord(n.position.y),
    })),
    edges: edges.map(e => {
      const edge = e.data?.edge ?? { id: e.id, kind: 'relates' as const, label: null, by: null };
      return {
        ...edge,
        label: normalLabel(edge.label),
        from: e.source,
        to: e.target,
        fromSide: isSide(e.sourceHandle) ? e.sourceHandle : null,
        toSide: isSide(e.targetHandle) ? e.targetHandle : null,
      };
    }),
  };
}

const sorted = (plan: PlanFile) => ({
  name: plan.name,
  // The person's settings: an outside switch or fine mark is a change too.
  autoLayout: plan.autoLayout ?? true,
  fine: plan.fine ?? [],
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
  /** An asset node's asset id. */
  asset?: string | null;
  /** A Lab direction node's direction id. */
  direction?: string | null;
  /** A camera plan's stop, whole, as `layout.json` keeps it. */
  stop?: LayoutStop | null;
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
    title: trimText(fields.title ?? '').slice(0, LIMITS.title),
    text: (fields.text ?? '').slice(0, LIMITS.text),
    image: fields.image ?? null,
    url: fields.url ? trimText(fields.url) : null,
    asset: fields.asset ?? null,
    direction: fields.direction ?? null,
    by: null,
    // Only a stop node carries one: the file leaves the field out otherwise.
    ...(fields.stop ? { stop: fields.stop } : {}),
  };
}

/** A flow node with some of its fields changed, inside the limits. */
export function withFields(flow: FlowNode, fields: NodeFields): FlowNode {
  const node = { ...flow.data.node, ...fields };
  node.title = node.title.slice(0, LIMITS.title);
  node.text = node.text.slice(0, LIMITS.text);
  return { ...flow, ariaLabel: nodeLabel(node), data: { node } };
}

/** A link between two nodes as a person draws it: they relate, or one stop leads to the next. */
export function newEdge(
  connection: Connection,
  nowMs: number,
  kind: EdgeKind = 'relates'
): FlowEdge | null {
  const { source, target } = connection;
  if (!source || !target || source === target) return null;
  const edge: PlanEdge = {
    id: newId('e', nowMs),
    from: source,
    to: target,
    fromSide: isSide(connection.sourceHandle) ? connection.sourceHandle : null,
    toSide: isSide(connection.targetHandle) ? connection.targetHandle : null,
    kind,
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
  const text = trimText(typed);
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

const UNTITLED: Record<NodeKind, string> = {
  note: 'Empty note',
  image: 'Picture',
  link: 'Link',
  asset: 'Linked asset',
  direction: 'Lab direction',
  stop: 'Camera stop',
};

/** A node's one line name, for the minimap, search and dialogs. */
export function nodeTitle(node: PlanNode): string {
  if (node.stop) return trimText(node.stop.name) || UNTITLED.stop;
  const title = trimText(node.title);
  if (title) return title;
  const first = trimText(node.text).split('\n')[0] ?? '';
  if (first) return first.length > 60 ? `${first.slice(0, 59)}...` : first;
  return UNTITLED[node.kind];
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

/** "9 nodes · 3 pictures · 2 links · 1 asset", leaving out what a canvas has none of. */
export function contentLine(plan: PlanSummary): string {
  const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  const parts = plan.camera
    ? [count(plan.stops, 'stop', 'stops')]
    : [count(plan.nodes, 'node', 'nodes')];
  if (plan.images) parts.push(count(plan.images, 'picture', 'pictures'));
  if (plan.links) parts.push(count(plan.links, 'link', 'links'));
  if (plan.assets) parts.push(count(plan.assets, 'asset', 'assets'));
  return parts.join(' · ');
}
