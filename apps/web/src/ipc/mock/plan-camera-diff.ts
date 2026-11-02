/**
 * The camera plan's reading in mock mode, close to the core's
 * (crates/maquette-model/src/plans/camera.rs and camera/diff.rs): the lines
 * of stop nodes joined by `leads-to` links, the main line Apply writes (the
 * one with the most of the person's stops), and the exact diff of two
 * lists of stops, a moved stop leaving its old place and coming in at its
 * new one.
 */
import type {
  CameraLine,
  DiffLine,
  FieldChange,
  LayoutStop,
  PlanFile,
  StopChange,
  StopDiff,
} from '../client';

export interface Chain extends CameraLine {
  people: number;
}

/** Every line of stop nodes in tour order, a stop with no link a line of its own. */
export function chainsOf(plan: PlanFile): Chain[] {
  const stops = plan.nodes.filter(n => n.kind === 'stop' && n.stop);
  const ids = new Set(stops.map(n => n.id));
  const leads = plan.edges.filter(
    e => e.kind === 'leads-to' && ids.has(e.from) && ids.has(e.to) && e.from !== e.to
  );
  const next = (id: string) => leads.filter(e => e.from === id).map(e => e.to);
  const prev = (id: string) => leads.filter(e => e.to === id).map(e => e.from);
  const seen = new Set<string>();
  const out: Chain[] = [];
  for (const start of [...stops].sort((a, b) => a.x - b.x || a.y - b.y)) {
    if (seen.has(start.id)) continue;
    const part = [start.id];
    seen.add(start.id);
    for (let i = 0; i < part.length; i += 1)
      for (const other of [...next(part[i]!), ...prev(part[i]!)])
        if (!seen.has(other)) {
          seen.add(other);
          part.push(other);
        }
    const heads = part.filter(id => prev(id).length === 0);
    const fork = part.find(id => next(id).length > 1 || prev(id).length > 1);
    let nodes = part;
    let problem: string | null = null;
    if (fork) problem = 'A stop leads to more than one stop. Keep one line: remove a link.';
    else if (heads.length !== 1) problem = 'The stops loop back to the start. Remove a link.';
    else {
      nodes = [heads[0]!];
      for (let to = next(heads[0]!)[0]; to; to = next(to)[0]) nodes.push(to);
    }
    const of = nodes.map(id => plan.nodes.find(n => n.id === id)!);
    const agents = new Set(of.flatMap(n => (n.by ? [n.by] : [])));
    const people = of.filter(n => n.by === null).length;
    const agent = people === 0 && agents.size === 1 ? [...agents][0]! : null;
    out.push({ nodes, problem, people, agent });
  }
  return out;
}

/** The line Apply writes: the most of the person's stops, then the longest, then the leftmost. */
export function mainOf(plan: PlanFile): Chain | null {
  const x = (c: Chain) => plan.nodes.find(n => n.id === c.nodes[0])?.x ?? 0;
  return (
    chainsOf(plan)
      .filter(c => c.people > 0)
      .sort((a, b) => b.people - a.people || b.nodes.length - a.nodes.length || x(a) - x(b))[0] ??
    null
  );
}

export const stopsOf = (plan: PlanFile, chain: Chain | null): LayoutStop[] =>
  (chain?.nodes ?? []).flatMap(id => {
    const stop = plan.nodes.find(n => n.id === id)?.stop;
    return stop ? [stop] : [];
  });

const num = (v: number | null) => {
  const text = (v ?? 0).toFixed(3).replace(/0+$/, '').replace(/\.$/, '');
  return text === '-0' ? '0' : text;
};
const vec3 = (v: readonly (number | null)[]) => `[${v.map(num).join(', ')}]`;
const models = (m: string[] | null) => (m ? m.join(', ') : 'what the view holds');

function fieldsOf(b: LayoutStop, a: LayoutStop): FieldChange[] {
  const pairs: [string, string, string][] = [
    ['name', b.name, a.name],
    ['position', vec3(b.position), vec3(a.position)],
    ['target', vec3(b.target), vec3(a.target)],
    ['fov', num(b.fov), num(a.fov)],
    ['models', models(b.models), models(a.models)],
  ];
  return pairs
    .filter(([, x, y]) => x !== y)
    .map(([field, before, after]) => ({ field, before, after }));
}

const lineText = (s: LayoutStop) =>
  `{ "id": ${JSON.stringify(s.id)}, "name": ${JSON.stringify(s.name)}, "position": ${vec3(s.position)}, "target": ${vec3(s.target)}, "fov": ${num(s.fov)}, "models": ${s.models ? JSON.stringify(s.models) : 'null'} }`;

/** The ids of the longest run of stops both lists hold in the same order. */
function keptOrder(before: string[], after: string[]): Set<string> {
  const n = before.length;
  const m = after.length;
  const best = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i -= 1)
    for (let j = m - 1; j >= 0; j -= 1)
      best[i]![j] =
        before[i] === after[j]
          ? best[i + 1]![j + 1]! + 1
          : Math.max(best[i + 1]![j]!, best[i]![j + 1]!);
  const kept = new Set<string>();
  for (let i = 0, j = 0; i < n && j < m;) {
    if (before[i] === after[j]) {
      kept.add(before[i]!);
      i += 1;
      j += 1;
    } else if (best[i + 1]![j]! >= best[i]![j + 1]!) i += 1;
    else j += 1;
  }
  return kept;
}

/** How `before` becomes `after`; `nodes` are the plan's stop nodes of `after`, in order. */
export function diffStops(
  before: LayoutStop[],
  after: LayoutStop[],
  nodes: string[] = []
): StopDiff {
  const bIds = before.map(s => s.id);
  const aIds = after.map(s => s.id);
  const kept = keptOrder(bIds, aIds);
  const out: StopDiff = { stops: [], lines: [], added: 0, removed: 0, moved: 0, changed: 0 };
  after.forEach((stop, i) => {
    const old = bIds.indexOf(stop.id);
    const fields = old >= 0 ? fieldsOf(before[old]!, stop) : [];
    const moved = old >= 0 && !kept.has(stop.id);
    out.added += old < 0 ? 1 : 0;
    out.moved += moved ? 1 : 0;
    out.changed += fields.length ? 1 : 0;
    const change: StopChange = {
      id: stop.id,
      name: stop.name,
      node: nodes[i] ?? null,
      from: old >= 0 ? old + 1 : null,
      to: i + 1,
      added: old < 0,
      removed: false,
      moved,
      fields,
    };
    out.stops.push(change);
  });
  before.forEach((stop, i) => {
    if (aIds.includes(stop.id)) return;
    out.removed += 1;
    out.stops.push({
      id: stop.id,
      name: stop.name,
      node: null,
      from: i + 1,
      to: null,
      added: false,
      removed: true,
      moved: false,
      fields: [],
    });
  });
  const line = (kind: DiffLine['kind'], s: LayoutStop, note: string | null): DiffLine => ({
    kind,
    text: lineText(s),
    note,
  });
  for (let i = 0, j = 0; i < before.length || j < after.length;) {
    const b = before[i];
    const a = after[j];
    if (b && !kept.has(b.id)) {
      const to = aIds.indexOf(b.id);
      out.lines.push(line('remove', b, to >= 0 ? `moves to ${to + 1}` : 'removed'));
      i += 1;
    } else if (a && !kept.has(a.id)) {
      const from = bIds.indexOf(a.id);
      out.lines.push(line('add', a, from >= 0 ? `moved from ${from + 1}` : 'new'));
      j += 1;
    } else if (a && b) {
      if (JSON.stringify(a) === JSON.stringify(b)) out.lines.push(line('same', a, null));
      else out.lines.push(line('remove', b, 'changed'), line('add', a, 'changed'));
      i += 1;
      j += 1;
    } else break;
  }
  return out;
}

/** A short stand in for the core's fingerprint. */
export function keyOf(value: unknown): string {
  let h = 0;
  for (const c of JSON.stringify(value)) h = (Math.imul(h, 31) + c.charCodeAt(0)) | 0;
  return (h >>> 0).toString(16);
}
