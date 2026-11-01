/**
 * The app's one recipe for waiting: a verb label, a count when the work
 * reports one, and how long the same wait took last time when it was measured.
 * The time is always a past run's, said as "last 12s" like the pipeline
 * runner's steps. No past run means no time: nothing here ever guesses.
 */
import { useEffect, useMemo, useRef } from 'react';
import { clock } from './clock';
import { formatElapsed } from './format';
import { readJson, writeJson } from './storage';

/** A wait shorter than this says nothing: it is over before anyone could read it. */
const WORTH_SAYING_MS = 2000;
const STORE_PREFIX = 'maquette.wait.';

/** "3 of 13". */
export function waitCount(done: number, total: number): string {
  return `${done} of ${total}`;
}

/** "last 12s", or null with no past run or one too short to mention. */
export function lastRunLabel(ms: number | null | undefined): string | null {
  return typeof ms === 'number' && ms >= WORTH_SAYING_MS ? `last ${formatElapsed(ms)}` : null;
}

/** How long the wait named `key` took last time, in milliseconds. */
export function readLastRun(key: string): number | null {
  const ms = readJson(STORE_PREFIX + key);
  return typeof ms === 'number' && Number.isFinite(ms) ? ms : null;
}

/** Keeps a finished wait's time. One too short to mention says nothing, and leaves a real time be. */
export function rememberRun(key: string, ms: number): void {
  if (ms < WORTH_SAYING_MS) return;
  writeJson(STORE_PREFIX + key, Math.round(ms));
}

/**
 * Times the wait named `key` while `active`, and says how long it took last
 * time. One that ends in `failed` or is left behind is not remembered: only a
 * wait that ran to its end is a past run. A new `ask` restarts the clock: the
 * question before it was dropped, and its time is not this answer's.
 */
export function useWait(key: string | null, active: boolean, failed = false, ask?: number) {
  const started = useRef<{ key: string; ask?: number; at: number } | null>(null);
  const estimate = useMemo(
    () => (key && active ? lastRunLabel(readLastRun(key)) : null),
    [key, active]
  );
  useEffect(() => {
    const run = started.current;
    if (active && key) {
      if (run?.key !== key || run.ask !== ask) started.current = { key, ask, at: clock.nowMs() };
      return;
    }
    if (run && !failed && run.key === key) rememberRun(run.key, clock.nowMs() - run.at);
    started.current = null;
  }, [key, active, failed, ask]);
  return estimate;
}

/** How long this wait took last time, read once per key; null when it never finished. */
export function usePastRun(key: string | null): string | null {
  return useMemo(() => (key ? lastRunLabel(readLastRun(key)) : null), [key]);
}
