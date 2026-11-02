/**
 * The canvases' change logs in mock mode, kept in memory as the core keeps
 * `plans/<id>.log.jsonl`: one entry per write with who made it, a short
 * summary and the operations, so the Changes panel lists them and undo
 * turns them around. Summaries follow the core's wording closely enough
 * for the panel (crates/maquette-model/src/plans/summary.rs).
 */
import type { ChangeMark, PlanLogLine, PlanNode, PlanOp } from '../client';
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
}

const KIND_NAME: Record<PlanNode['kind'], string> = {
  note: 'note',
  image: 'picture',
  link: 'link',
  asset: 'asset',
  direction: 'Lab direction',
};

const nameOf = (node: PlanNode) => node.title || node.text.split('\n')[0]?.slice(0, 48) || node.id;

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
      return [...all]
        .reverse()
        .slice(0, limit)
        .map(e => ({
          id: e.id,
          at: e.at,
          by: e.by,
          run: e.run,
          summary: e.summary,
          mark: e.mark,
          undone: undone.has(e.id),
          undoable: !undone.has(e.id) && e.ops.length > 0,
        }));
    },
    drop: (project: unknown, id: unknown) => logs.delete(`${String(project)}/${String(id)}`),
  };
}
