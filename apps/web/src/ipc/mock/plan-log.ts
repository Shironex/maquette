/**
 * The canvases' change logs in mock mode, kept in memory as the core keeps
 * `plans/<id>.log.jsonl`: one entry per write with who made it, a short
 * summary and the operations, so the Changes panel lists them and undo
 * turns them around. Summaries follow the core's wording closely enough
 * for the panel (crates/maquette-model/src/plans/summary.rs).
 */
import type { ChangeMark, KeepNeed, PlanLogLine, PlanNode, PlanOp } from '../client';
import { clock } from '@/lib/clock';

export interface MockLogEntry {
  id: string;
  at: string;
  by: string | null;
  run: string | null;
  summary: string;
  mark: ChangeMark;
  ops: PlanOp[];
  undoes: string[];
  keeps?: string[];
}

/** Whether `op` changed a person's content: their node or link, or an agent node they wrote in last. */
const touchesPerson = (op: PlanOp) =>
  op.op === 'set-node' || op.op === 'remove-node'
    ? (op.op === 'set-node' ? op.before : op.node).by === null ||
      Boolean((op.op === 'set-node' ? op.before : op.node).personWrote)
    : op.op === 'set-edge' || op.op === 'remove-edge'
      ? (op.op === 'set-edge' ? op.before : op.edge).by === null
      : false;

const FIELDS = [
  'kind',
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
] as const;

/** `node:id:field` or `link:id:*` for each part `op` changed, as the core's undo order reads them. */
const partsOf = (op: PlanOp): string[] => {
  if (op.op === 'set-node')
    return FIELDS.filter(f => op.before[f] !== op.after[f]).map(f => `node:${op.after.id}:${f}`);
  if (op.op === 'add-node' || op.op === 'remove-node') return [`node:${op.node.id}:*`];
  if (op.op === 'set-camera') return ['camera::*'];
  return [`link:${op.op === 'set-edge' ? op.after.id : op.edge.id}:*`];
};

const meets = (a: string[], b: string[]) =>
  a.some(x =>
    b.some(y => {
      const [ka, ia, fa] = x.split(':');
      const [kb, ib, fb] = y.split(':');
      return ka === kb && ia === ib && (fa === fb || fa === '*' || fb === '*');
    })
  );

/** The earlier held agent changes a Keep of `ids` must take along, as the core's `keep_needs`. */
function needsOf(all: MockLogEntry[], ids: string[]): KeepNeed[] {
  const released = new Set(all.flatMap(e => [...e.undoes, ...(e.keeps ?? [])]));
  const held = (e: MockLogEntry) =>
    e.by !== null &&
    !e.undoes.length &&
    !e.keeps?.length &&
    !released.has(e.id) &&
    e.ops.some(touchesPerson);
  const found: MockLogEntry[] = [];
  const open = all.filter(e => ids.includes(e.id));
  for (let top = open.pop(); top; top = open.pop()) {
    const mine = top.ops.flatMap(partsOf);
    for (const earlier of all.slice(0, all.indexOf(top))) {
      const seen = ids.includes(earlier.id) || found.includes(earlier);
      if (!seen && held(earlier) && meets(mine, earlier.ops.flatMap(partsOf))) {
        found.push(earlier);
        open.push(earlier);
      }
    }
  }
  return found.map(e => ({
    id: e.id,
    at: e.at,
    summary: e.summary,
    parts: [
      ...new Set(e.ops.flatMap(partsOf).map(p => p.split(':')[2]!.replace('*', 'whole node'))),
    ],
  }));
}

const KIND_NAME: Record<PlanNode['kind'], string> = {
  note: 'note',
  image: 'picture',
  link: 'link',
  asset: 'asset',
  direction: 'Lab direction',
  stop: 'camera stop',
};

/** `text` cut to `max` characters at the end of a word, with an ellipsis, as the core's summaries are. */
export function clipWords(text: string, max = 60): string {
  const chars = [...text];
  if (chars.length <= max) return text;
  const head = chars.slice(0, max - 1).join('');
  const at = head.search(/\s\S*$/);
  const cut =
    at >= 0 && [...head.slice(0, at)].length >= Math.floor(max / 3) ? head.slice(0, at) : head;
  return `${cut.replace(/[\s,.]+$/, '')}\u2026`;
}

const nameOf = (node: PlanNode) =>
  clipWords(node.title.trim() || node.text.trim().split('\n')[0]?.trim() || node.id);

const counted = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;

/** "Added note: Dusk light", or "Added 2 notes, moved 1 note". */
export function summaryOf(ops: readonly PlanOp[]): { summary: string; mark: ChangeMark } {
  const added = ops.flatMap(o => (o.op === 'add-node' ? [o.node] : []));
  const removed = ops.flatMap(o => (o.op === 'remove-node' ? [o.node] : []));
  const sets = ops.flatMap(o => (o.op === 'set-node' ? [o] : []));
  const moved = sets.filter(o => o.before.x !== o.after.x || o.before.y !== o.after.y);
  const edited = sets.filter(o => !moved.includes(o));
  const links = ops.filter(o => o.op.endsWith('-edge')).length;
  const one = (verb: string, node: PlanNode) =>
    `${verb.toLowerCase()} ${KIND_NAME[node.kind]}: ${nameOf(node)}`;
  const parts: string[] = [];
  if (added.length)
    parts.push(
      added.length === 1 && added[0]
        ? one('Added', added[0])
        : `Added ${counted(added.length, 'node')}`
    );
  if (edited.length)
    parts.push(
      edited.length === 1 && edited[0]
        ? one('Edited', edited[0].after)
        : `edited ${counted(edited.length, 'node')}`
    );
  if (moved.length) parts.push(`moved ${counted(moved.length, 'node')}`);
  if (removed.length) parts.push(`removed ${counted(removed.length, 'node')}`);
  if (!parts.length && links) parts.push(`Changed ${counted(links, 'link')}`);
  const text = parts.join(', ');
  const kinds = [added, edited, moved, removed].filter(k => k.length).length;
  const mark: ChangeMark =
    kinds > 1
      ? 'mixed'
      : added.length
        ? 'add'
        : edited.length
          ? 'edit'
          : moved.length
            ? 'move'
            : removed.length
              ? 'remove'
              : 'link';
  return { summary: text.charAt(0).toUpperCase() + text.slice(1), mark };
}

let seq = 0;

export function createMockLogs() {
  const logs = new Map<string, MockLogEntry[]>();
  const of = (project: unknown, id: unknown) => {
    const key = `${String(project)}/${String(id)}`;
    let found = logs.get(key);
    if (!found) {
      found = [];
      logs.set(key, found);
    }
    return found;
  };
  return {
    entries: of,
    needs: (project: unknown, id: unknown, ids: string[]) => needsOf(of(project, id), ids),
    append: (
      project: unknown,
      id: unknown,
      entry: Omit<MockLogEntry, 'id' | 'at' | 'undoes'> & { at?: string; undoes?: string[] }
    ) => {
      seq += 1;
      const full: MockLogEntry = {
        id: `c-${seq.toString(36)}`,
        at: entry.at ?? clock.now().toISOString(),
        undoes: [],
        ...entry,
      };
      of(project, id).push(full);
      return full;
    },
    /** Newest first, as `plan_log` answers. */
    lines: (project: unknown, id: unknown, limit: number): PlanLogLine[] => {
      const all = of(project, id);
      const undone = new Set(all.flatMap(e => e.undoes));
      const kept = new Set(all.flatMap(e => e.keeps ?? []));
      return [...all]
        .reverse()
        .slice(0, limit)
        .map(e => ({
          keepable:
            e.by !== null &&
            !e.undoes.length &&
            !undone.has(e.id) &&
            !kept.has(e.id) &&
            e.ops.some(touchesPerson),
          kept: kept.has(e.id),
          id: e.id,
          at: e.at,
          by: e.by,
          run: e.run,
          summary: e.summary,
          mark: e.mark,
          undone: undone.has(e.id),
          undoable: !undone.has(e.id) && e.ops.length > 0 && !e.undoes.length,
        }));
    },
    drop: (project: unknown, id: unknown) => logs.delete(`${String(project)}/${String(id)}`),
  };
}
