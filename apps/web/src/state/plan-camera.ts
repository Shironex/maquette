/**
 * The camera plan's rules on the canvas: stop nodes joined by `leads-to`
 * links in tour order, as the core reads them (crates/maquette-model/src/
 * plans/camera.rs). Which line Apply writes comes from the core's view;
 * these helpers change the flow the way a person edits it: a stop removed
 * from a line joins its neighbours, a stop added goes at the end of the
 * main line, a stop moves earlier or later, and an agent's proposal can be
 * taken as the main line's order.
 */
import type { CameraView, FieldChange, LayoutStop, PlanEdge } from '@/ipc/client';
import { flowEdgeOf, flowNodeOf, newId, newNode, type FlowEdge, type FlowNode } from './plans';

export const CAMERA_PLAN_ID = 'camera-plan';
/** How far apart stops of a line are laid, as the core lays them. */
export const STEP_X = 280;
/** The field of view a new stop gets, as `layout.json`'s default. */
export const DEFAULT_FOV = 35;
/** The most stops a tour holds (the core's `MAX_STOPS`). */
export const MAX_STOPS = 24;

const isLead = (e: FlowEdge) => e.data?.edge.kind === 'leads-to';
const stopOf = (n: FlowNode | undefined) => n?.data.node.stop ?? null;

/** A `leads-to` link from one stop to the next, right side to left side. */
export function leadEdge(from: string, to: string, nowMs: number): FlowEdge {
  const edge: PlanEdge = {
    id: newId('e', nowMs),
    from,
    to,
    fromSide: 'right',
    toSide: 'left',
    kind: 'leads-to',
    label: null,
    by: null,
  };
  return flowEdgeOf(edge);
}

/** Each main line node's place in the tour, from 1. */
export function placesOf(view: CameraView | null): Map<string, number> {
  return new Map((view?.main ?? []).map((id, i) => [id, i + 1]));
}

/** The line holding `node` in the view, with its agent: the main line or a proposal. */
export function lineOf(view: CameraView | null, node: string) {
  if (!view) return null;
  if (view.main.includes(node)) return { nodes: view.main, agent: null, main: true };
  const line = view.lines.find(l => l.nodes.includes(node));
  return line ? { nodes: line.nodes, agent: line.agent, main: false } : null;
}

/**
 * The flow with the stop nodes `removed` gone: where a removed stop sat
 * between two others on a line, those two are joined, so a line never
 * splits in two by a removal.
 */
export function healed(
  before: readonly FlowEdge[],
  after: readonly FlowEdge[],
  removed: readonly string[],
  nowMs: number
): FlowEdge[] {
  const gone = new Set(removed);
  const leads = before.filter(isLead);
  const out = [...after];
  for (const id of removed) {
    let from = leads.find(e => e.target === id)?.source;
    let to = leads.find(e => e.source === id)?.target;
    // Walk past neighbours removed in the same change.
    while (from && gone.has(from)) from = leads.find(e => e.target === from)?.source;
    while (to && gone.has(to)) to = leads.find(e => e.source === to)?.target;
    const joined = out.some(e => isLead(e) && e.source === from && e.target === to);
    if (from && to && from !== to && !joined) out.push(leadEdge(from, to, nowMs));
  }
  return out;
}

/** The main line's nodes laid one step apart from its first, joined in `order`. */
function relaid(
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[],
  order: readonly string[],
  nowMs: number
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const head = nodes.find(n => n.id === order[0]);
  const at = new Map(order.map((id, i) => [id, i]));
  const inLine = (id: string) => at.has(id);
  const placed = nodes.map(n => {
    const i = at.get(n.id);
    if (i === undefined || !head) return n;
    const position = { x: head.position.x + i * STEP_X, y: head.position.y };
    return { ...n, position, data: { node: { ...n.data.node, ...position } } };
  });
  const kept = edges.filter(e => !(isLead(e) && inLine(e.source) && inLine(e.target)));
  const links = order.slice(1).map((id, i) => leadEdge(order[i]!, id, nowMs + i));
  return { nodes: placed, edges: [...kept, ...links] };
}

/** The main line with the stop `id` moved `delta` places, or null when it can not move. */
export function moved(
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[],
  main: readonly string[],
  id: string,
  delta: -1 | 1,
  nowMs: number
) {
  const from = main.indexOf(id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= main.length) return null;
  const order = [...main];
  [order[from], order[to]] = [order[to]!, order[from]!];
  return relaid(nodes, edges, order, nowMs);
}

/** A stop id from its name, unique among `taken`: "Lantern close-up" is `lantern-close-up`. */
export function stopId(name: string, taken: ReadonlySet<string>): string {
  const base =
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 48) || 'stop';
  let id = base;
  for (let n = 2; taken.has(id); n += 1) id = `${base}-${n}`;
  return id;
}

/** Every stop id the flow holds. */
export function stopIds(nodes: readonly FlowNode[]): Set<string> {
  return new Set(nodes.flatMap(n => (n.data.node.stop ? [n.data.node.stop.id] : [])));
}

/**
 * A new stop at the end of the main line, looking where the last one
 * looks from a step aside, or the flow's first stop line when the plan has
 * none yet. Its id and name are unique.
 */
export function withNewStop(
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[],
  main: readonly string[],
  nowMs: number
): { nodes: FlowNode[]; edges: FlowEdge[]; id: string } {
  const lastId = main[main.length - 1];
  const last = nodes.find(n => n.id === lastId);
  const was = stopOf(last);
  const taken = stopIds(nodes);
  let count = taken.size + 1;
  while (taken.has(stopId(`Stop ${count}`, taken)) && count < 999) count += 1;
  const name = `Stop ${count}`;
  const stop: LayoutStop = {
    id: stopId(name, taken),
    name,
    position: was ? [(was.position[0] ?? 0) + 1, was.position[1], was.position[2]] : [0, 1.6, 6],
    target: was ? [...was.target] : [0, 1, 0],
    fov: was?.fov ?? DEFAULT_FOV,
    models: null,
  };
  const at = last ? { x: last.position.x + STEP_X, y: last.position.y } : { x: 0, y: 0 };
  const node = { ...newNode('stop', at, { stop }, nowMs), title: '' };
  const fresh = flowNodeOf(node);
  const links = last ? [leadEdge(last.id, node.id, nowMs)] : [];
  return { nodes: [...nodes, fresh], edges: [...edges, ...links], id: node.id };
}

/**
 * The main line set to the order and settings of the proposal `line`: a
 * stop the main line has keeps its node and takes the proposal's
 * settings, a new one gets a node of the person's, and main line stops the
 * proposal left out go. The proposal's own nodes stay for the agent.
 */
export function adopted(
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[],
  main: readonly string[],
  line: readonly string[],
  nowMs: number
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const byId = new Map(nodes.map(n => [n.id, n]));
  const mainByStop = new Map(
    main.flatMap(id => {
      const stop = stopOf(byId.get(id));
      return stop ? [[stop.id, id] as const] : [];
    })
  );
  const head = byId.get(main[0] ?? '');
  const order: string[] = [];
  let next = nodes.map(n => n);
  for (const [i, id] of line.entries()) {
    const stop = stopOf(byId.get(id));
    if (!stop) continue;
    const mine = mainByStop.get(stop.id);
    if (mine) {
      next = next.map(n =>
        n.id === mine ? { ...n, data: { node: { ...n.data.node, stop } } } : n
      );
      order.push(mine);
    } else {
      const at = { x: (head?.position.x ?? 0) + i * STEP_X, y: head?.position.y ?? 0 };
      const node = { ...newNode('stop', at, { stop }, nowMs + i), title: '' };
      next.push(flowNodeOf(node));
      order.push(node.id);
    }
  }
  const dropped = new Set(main.filter(id => !order.includes(id)));
  const kept = next.filter(n => !dropped.has(n.id));
  const keptEdges = edges.filter(e => !dropped.has(e.source) && !dropped.has(e.target));
  return relaid(kept, keptEdges, order, nowMs);
}

/** The flow without the proposal `line`'s nodes and their links. */
export function discarded(
  nodes: readonly FlowNode[],
  edges: readonly FlowEdge[],
  line: readonly string[]
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const gone = new Set(line);
  return {
    nodes: nodes.filter(n => !gone.has(n.id)),
    edges: edges.filter(e => !gone.has(e.source) && !gone.has(e.target)),
  };
}

/** "Moon: 2.4, 1.3, 3.4" for a position or target, two decimals at most. */
export const vec = (v: readonly (number | null)[]) =>
  v.map(n => Math.round((n ?? 0) * 100) / 100).join(', ');

/** "1 stop added · 3 moved · 0 removed · 2 changed", as the core counts them. */
export function countsLine(d: { added: number; moved: number; removed: number; changed: number }) {
  const stops = (n: number) => `${n} stop${n === 1 ? '' : 's'}`;
  return `${stops(d.added)} added · ${d.moved} moved · ${d.removed} removed · ${d.changed} changed`;
}

/** "; fov 35 to 28" in full, or ", fov" short, for a stop's changed settings; empty when none. */
export function fieldsLine(fields: readonly FieldChange[], full: boolean): string {
  if (!fields.length) return '';
  if (!full) return `, ${fields.map(f => f.field).join(', ')}`;
  return `; ${fields.map(f => `${f.field} ${f.before} to ${f.after}`).join('; ')}`;
}
