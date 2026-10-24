/**
 * Sorts pipeline log lines so the useful ones stand out: failures, warnings,
 * step milestones, results and our own tagged script lines, while the
 * tools' chatter fades.
 */
import type { LogEntry } from '@/ipc/client';

export type LogKind =
  'error' | 'warning' | 'waiting' | 'milestone' | 'tagged' | 'result' | 'noise' | 'plain';

export interface LogClass {
  kind: LogKind;
  /** The `[tag]` of a script line, without brackets. */
  tag?: string;
  /** The line after the tag; the whole line without one. */
  body: string;
}

const ERROR = /traceback|error\b|\bfailed\b|exit(?:ed)? with code|exit code/i;
const WARNING = /^\W*warning\b|\|\s*warning:|\bwarning:/i;
const WAITING = /waits for approval|waiting for approval/i;
const MILESTONE = /\bdone in \d|^run \d+ done|\bship approved\b/i;
const TAG = /^\[([a-z][a-z0-9_]*)\]\s*(.*)$/;
const RESULT = /→|^\W*wrote\b|\bsaved\b/i;
const NOISE = [
  /\|\s*info:/i,
  /^\W*(?:info:\s*)?prune: removed types/i,
  /^blender quit/i,
  /^blender \d+\.\d+/i,
  /^\W*info: baking map saved/i,
  /\b(?:draco|meshoptimizer)\b.*\b(?:available|unavailable|found|missing|loaded)\b/i,
];

/** Which kind of line this is. `level` is what the runner already decided. */
export function classifyLog(text: string, level: LogEntry['level'] = 'info'): LogClass {
  const line = text.trim();
  const tagged = TAG.exec(line);
  const body = tagged ? (tagged[2] ?? '') : text;
  const tag = tagged?.[1];
  const make = (kind: LogKind): LogClass => (tag ? { kind, tag, body } : { kind, body });
  if (level === 'error' || ERROR.test(line)) return make('error');
  if (level === 'warn' || WARNING.test(line)) return make('warning');
  if (WAITING.test(line)) return make('waiting');
  if (level === 'ok' || MILESTONE.test(line)) return make('milestone');
  if (NOISE.some(rx => rx.test(line))) return make('noise');
  if (tag) return { kind: 'tagged', tag, body };
  if (RESULT.test(line)) return make('result');
  return make('plain');
}

const SIZE = /\d[\d.,]*\s?(?:[KMG]i?)?B\b/gi;

/** A result line split around its last arrow, with the final size marked. */
export interface SizeChange {
  before: string;
  /** What comes after the arrow up to the final size. */
  between: string;
  size: string;
  after: string;
}

/** Splits "a (1.27 MB) → b (653 KB)" so the size it ends at can stand out. */
export function sizeChange(text: string): SizeChange | null {
  const arrow = text.lastIndexOf('→');
  if (arrow < 0) return null;
  const before = text.slice(0, arrow + 1);
  const rest = text.slice(arrow + 1);
  const sizes = [...rest.matchAll(SIZE)];
  const last = sizes[sizes.length - 1];
  if (!last) return null;
  const start = last.index;
  return {
    before,
    between: rest.slice(0, start),
    size: last[0],
    after: rest.slice(start + last[0].length),
  };
}
