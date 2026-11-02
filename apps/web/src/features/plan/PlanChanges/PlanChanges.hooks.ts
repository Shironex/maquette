import { useMemo, useState } from 'react';
import { useNow } from '@/lib/useNow';
import { runsOf, whenOf, type LogFilter, type LogRun } from '@/state/plan-log';
import { usePlanLogStore } from '@/state/use-plan-log-store';

/** A minute is fine for "12:40" and "Yesterday". */
const CLOCK_MS = 60_000;

/**
 * The Changes panel: the open canvas's change log in runs, filtered to all,
 * agents or mine, an agent's run open and a person's folded, and undo of a
 * run or one change. One undo at a time.
 */
export function usePlanChanges() {
  const lines = usePlanLogStore(s => s.lines);
  const error = usePlanLogStore(s => s.error);
  const filter = usePlanLogStore(s => s.filter);
  const setFilter = usePlanLogStore(s => s.setFilter);
  const setPanel = usePlanLogStore(s => s.setPanel);
  const undoLines = usePlanLogStore(s => s.undo);
  const now = useNow(CLOCK_MS);
  const runs = useMemo(() => runsOf(lines, filter), [lines, filter]);
  const [folded, setFolded] = useState<Record<string, boolean>>({});
  const busy = usePlanLogStore(s => s.undoing);
  const isOpen = (run: LogRun) => !(folded[run.key] ?? run.by === null);
  return {
    runs,
    error,
    filter,
    setFilter: (next: LogFilter) => setFilter(next),
    busy,
    when: (at: string) => whenOf(at, new Date(now)),
    isOpen,
    toggle: (run: LogRun) => setFolded(f => ({ ...f, [run.key]: isOpen(run) })),
    undo: (ids: string[]) => void undoLines(ids),
    close: () => setPanel(false),
  };
}
