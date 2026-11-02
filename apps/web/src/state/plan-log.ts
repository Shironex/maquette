/**
 * The Changes panel's rules: the open canvas's change log grouped into
 * runs (an agent's calls in one run, or one writer's changes close in
 * time), filtered to all, agents or mine, and the agent change worth a
 * toast when it lands.
 */
import type { PlanLogLine } from '@/ipc/client';

export type LogFilter = 'all' | 'agents' | 'mine';

/** One writer's changes, newest first: an agent's run, or a person's changes close in time. */
export interface LogRun {
  key: string;
  /** The agent's name, or null for a person. */
  by: string | null;
  lines: PlanLogLine[];
  /** The lines of it that can still be undone. */
  undoable: string[];
}

/** Changes by one writer this close together, without a run id, make one group. */
const SAME_RUN_MS = 10 * 60 * 1000;

const ms = (at: string) => Date.parse(at) || 0;

/** `lines` (newest first) as runs, newest first. */
export function runsOf(lines: readonly PlanLogLine[], filter: LogFilter = 'all'): LogRun[] {
  const shown = lines.filter(
    l => filter === 'all' || (filter === 'agents' ? l.by !== null : l.by === null)
  );
  const runs: LogRun[] = [];
  for (const line of shown) {
    const last = runs.at(-1);
    const oldest = last?.lines.at(-1);
    const same =
      last &&
      oldest &&
      last.by === line.by &&
      (line.run !== null
        ? oldest.run === line.run
        : oldest.run === null && ms(oldest.at) - ms(line.at) < SAME_RUN_MS);
    if (same) last.lines.push(line);
    else runs.push({ key: line.id, by: line.by, lines: [line], undoable: [] });
  }
  for (const run of runs) run.undoable = run.lines.filter(l => l.undoable).map(l => l.id);
  return runs;
}

/** The newest agent change made after `since` (RFC 3339) that can still be undone, for the toast. */
export function freshAgentLine(lines: readonly PlanLogLine[], since: string): PlanLogLine | null {
  const newest = lines.find(l => l.by !== null && l.mark !== 'undo');
  return newest && newest.at > since && newest.undoable ? newest : null;
}

/** The agent's run `line` belongs to: its line ids that can be undone. */
export function runIds(lines: readonly PlanLogLine[], line: PlanLogLine): string[] {
  const run = runsOf(lines).find(r => r.lines.some(l => l.id === line.id));
  return run?.undoable ?? [line.id];
}

/** How many agent changes came after `since`, for the Changes button's count. */
export function unseenCount(lines: readonly PlanLogLine[], since: string): number {
  return lines.filter(l => l.by !== null && l.at > since).length;
}

/** "12:40", or "Yesterday", or "Oct 30" for older ones, in the reader's time. */
export function whenOf(at: string, now: Date): string {
  const date = new Date(at);
  if (Number.isNaN(date.getTime())) return '';
  const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((day(now) - day(date)) / 86_400_000);
  if (days === 0) {
    const hh = String(date.getHours()).padStart(2, '0');
    const mm = String(date.getMinutes()).padStart(2, '0');
    return `${hh}:${mm}`;
  }
  if (days === 1) return 'Yesterday';
  return date.toLocaleDateString('en', { month: 'short', day: 'numeric' });
}
