/**
 * Canvas changes as operations by node and edge id, the same ones the core
 * applies (crates/maquette-model/src/plans/ops.rs): the view saves the
 * operations between the canvas it last had from the file and what it
 * shows, so an agent's write made meanwhile is never written over; a
 * canvas changed outside is merged with the person's unsaved edits by
 * [`rebase`]; undo and redo turn a person's operations around.
 */
import type { PlanEdge, PlanFile, PlanNode, PlanOp } from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';

/** `mine`: the writer's values win. `careful`: only what still holds the expected value changes. */
export type Merge = 'mine' | 'careful';

const NODE_FIELDS = [
  'x',
  'y',
  'width',
  'title',
  'text',
  'image',
  'url',
  'asset',
  'direction',
  'stop',
] as const satisfies readonly (keyof PlanNode)[];
const EDGE_FIELDS = [
  'fromSide',
  'toSide',
  'kind',
  'label',
] as const satisfies readonly (keyof PlanEdge)[];

/** The operations that turn `from` into `to`, in the core's order. */
export function diffPlans(from: PlanFile, to: PlanFile): PlanOp[] {
  const fromNodes = new Map(from.nodes.map(n => [n.id, n]));
  const toNodes = new Map(to.nodes.map(n => [n.id, n]));
  const fromEdges = new Map(from.edges.map(e => [e.id, e]));
  const toEdges = new Map(to.edges.map(e => [e.id, e]));
  const ops: PlanOp[] = [];
  for (const edge of from.edges) if (!toEdges.has(edge.id)) ops.push({ op: 'remove-edge', edge });
  for (const node of from.nodes) if (!toNodes.has(node.id)) ops.push({ op: 'remove-node', node });
  for (const node of to.nodes) {
    const before = fromNodes.get(node.id);
    if (!before) ops.push({ op: 'add-node', node });
    else if (!deepEqual(before, node)) ops.push({ op: 'set-node', before, after: node });
  }
  for (const edge of to.edges) {
    const before = fromEdges.get(edge.id);
    if (!before) ops.push({ op: 'add-edge', edge });
    else if (!deepEqual(before, edge)) ops.push({ op: 'set-edge', before, after: edge });
  }
  return ops;
}

/** The operations that undo `ops`: each turned around, last first. */
export function invertOps(ops: readonly PlanOp[]): PlanOp[] {
  return [...ops].reverse().map((op): PlanOp => {
    switch (op.op) {
      case 'add-node':
        return { op: 'remove-node', node: op.node };
      case 'remove-node':
        return { op: 'add-node', node: op.node };
      case 'set-node':
        return { op: 'set-node', before: op.after, after: op.before };
      case 'add-edge':
        return { op: 'remove-edge', edge: op.edge };
      case 'remove-edge':
        return { op: 'add-edge', edge: op.edge };
      case 'set-edge':
        return { op: 'set-edge', before: op.after, after: op.before };
      case 'set-camera':
        return { op: 'set-camera', before: op.after, after: op.before };
    }
  });
}

/** Copies the fields `before` and `after` disagree on; false when one was left because it changed meanwhile. */
function mergeFields<T extends object, K extends keyof T>(
  current: T,
  before: T,
  after: T,
  fields: readonly K[],
  merge: Merge
): { next: T; whole: boolean } {
  const next = { ...current };
  let whole = true;
  for (const f of fields) {
    if (deepEqual(before[f], after[f])) continue;
    if (merge === 'mine' || deepEqual(current[f], before[f])) next[f] = after[f];
    else whole = false;
  }
  return { next, whole };
}

/** Applies `ops` to `plan` in order, as the core does; ops that no longer fit are skipped and counted. */
export function applyOps(
  plan: PlanFile,
  ops: readonly PlanOp[],
  merge: Merge
): { plan: PlanFile; skipped: number } {
  let nodes = [...plan.nodes];
  let edges = [...plan.edges];
  let camera = plan.camera;
  let skipped = 0;
  const nodeAt = (id: string) => nodes.findIndex(n => n.id === id);
  const edgeAt = (id: string) => edges.findIndex(e => e.id === id);
  const addEdge = (edge: PlanEdge) => {
    const joins = nodeAt(edge.from) >= 0 && nodeAt(edge.to) >= 0;
    const at = edgeAt(edge.id);
    if (!joins || (at >= 0 && merge === 'careful')) return false;
    if (at >= 0) edges[at] = edge;
    else edges.push(edge);
    return true;
  };
  const one = (op: PlanOp): boolean => {
    switch (op.op) {
      case 'add-node': {
        const at = nodeAt(op.node.id);
        if (at >= 0 && merge === 'careful') return false;
        if (at >= 0) nodes[at] = op.node;
        else nodes.push(op.node);
        return true;
      }
      case 'remove-node': {
        const at = nodeAt(op.node.id);
        if (at < 0 || (merge === 'careful' && !deepEqual(nodes[at], op.node))) return false;
        const linked = edges.some(e => e.from === op.node.id || e.to === op.node.id);
        // Careful (an undo): a node someone linked since stays, and so do their links.
        if (merge === 'careful' && linked) return false;
        nodes = nodes.filter(n => n.id !== op.node.id);
        edges = edges.filter(e => e.from !== op.node.id && e.to !== op.node.id);
        return true;
      }
      case 'set-node': {
        const at = nodeAt(op.after.id);
        const current = nodes[at];
        if (!current) {
          if (merge === 'careful') return false;
          nodes.push(op.after);
          return true;
        }
        const { next, whole } = mergeFields(current, op.before, op.after, NODE_FIELDS, merge);
        nodes[at] = next;
        return whole;
      }
      case 'add-edge':
        return addEdge(op.edge);
      case 'remove-edge': {
        const at = edgeAt(op.edge.id);
        if (at < 0 || (merge === 'careful' && !deepEqual(edges[at], op.edge))) return false;
        edges = edges.filter(e => e.id !== op.edge.id);
        return true;
      }
      case 'set-edge': {
        const at = edgeAt(op.after.id);
        const current = edges[at];
        if (!current) return merge === 'mine' ? addEdge(op.after) : false;
        const { next, whole } = mergeFields(current, op.before, op.after, EDGE_FIELDS, merge);
        edges[at] = next;
        return whole;
      }
      case 'set-camera':
        // Only a rebase or an Apply moves the base; an undo leaves one changed since.
        if (merge === 'careful' && !deepEqual(camera ?? null, op.before)) return false;
        camera = op.after;
        return true;
    }
  };
  for (const op of ops) if (!one(op)) skipped += 1;
  return { plan: { ...plan, nodes, edges, camera }, skipped };
}

/**
 * The node and edge ids both sides changed since `base` in ways that
 * disagree: the same field to different values, or one side editing what
 * the other removed.
 */
export function conflictsOf(base: PlanFile, mine: PlanFile, theirs: PlanFile): string[] {
  const theirNodes = new Map(theirs.nodes.map(n => [n.id, n]));
  const theirEdges = new Map(theirs.edges.map(e => [e.id, e]));
  const baseNodes = new Map(base.nodes.map(n => [n.id, n]));
  const baseEdges = new Map(base.edges.map(e => [e.id, e]));
  const clash = <T, K extends keyof T>(before: T, after: T, their: T, fields: readonly K[]) =>
    fields.some(
      f =>
        !deepEqual(before[f], after[f]) &&
        !deepEqual(their[f], before[f]) &&
        !deepEqual(their[f], after[f])
    );
  const ids = new Set<string>();
  for (const op of diffPlans(base, mine)) {
    if (op.op === 'set-node') {
      const their = theirNodes.get(op.after.id);
      if (!their || clash(op.before, op.after, their, NODE_FIELDS)) ids.add(op.after.id);
    } else if (op.op === 'remove-node') {
      const their = theirNodes.get(op.node.id);
      if (their && !deepEqual(their, baseNodes.get(op.node.id))) ids.add(op.node.id);
    } else if (op.op === 'set-edge') {
      const their = theirEdges.get(op.after.id);
      if (!their || clash(op.before, op.after, their, EDGE_FIELDS)) ids.add(op.after.id);
    } else if (op.op === 'remove-edge') {
      const their = theirEdges.get(op.edge.id);
      if (their && !deepEqual(their, baseEdges.get(op.edge.id))) ids.add(op.edge.id);
    }
  }
  return [...ids];
}

/**
 * The person's edits since `base` laid over `theirs` (the file as changed
 * outside). Nothing either side did alone is lost; where both changed the
 * same thing, `winner` decides.
 */
export function rebase(
  base: PlanFile,
  mine: PlanFile,
  theirs: PlanFile,
  winner: 'mine' | 'theirs'
): PlanFile {
  const ops = diffPlans(base, mine);
  return applyOps(theirs, ops, winner === 'mine' ? 'mine' : 'careful').plan;
}
