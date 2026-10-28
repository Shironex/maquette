/**
 * Codex usage in mock mode (`?codex=`):
 * - `ok` (default): 16% of the five hour window and 3% of the week used.
 * - `low`: 86% of the five hour window used, amber in the status bar.
 * - `limit`: the five hour window is full until the `?lab=usage` jobs'
 *   time, red; Limit lifted, retry now finds it still holds.
 * - `lifted`: reads as `limit` until Limit lifted, retry now, whose read
 *   finds it lifted and runs the held jobs again.
 * - `unknown`: this Codex cannot say.
 */
import type { CodexUsage, ConceptJob, LabView, UsageLift } from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';

const HOUR = 3_600_000;
/** The `?lab=usage` jobs wait 37 minutes. */
const LIMIT_LIFTS_IN = 37 * 60_000;

function window(usedPercent: number, windowMins: number, resetsAtMs: number) {
  return { usedPercent, windowMins, resetsAtMs };
}

export function mockUsage(state: string | null, now: number): CodexUsage {
  const base: CodexUsage = {
    known: true,
    reason: null,
    readAtMs: now,
    primary: window(16, 300, now + 3.2 * HOUR),
    secondary: window(3, 10080, now + 4.5 * 24 * HOUR),
    plan: 'plus',
    limitReached: null,
    ordinaryUsageAllowed: true,
    credits: { hasCredits: false, unlimited: false, balance: '0' },
    resetCredits: 0,
  };
  switch (state) {
    case 'low':
      return {
        ...base,
        primary: window(86, 300, now + 1.4 * HOUR),
        secondary: window(41, 10080, now + 2 * 24 * HOUR),
      };
    case 'limit':
    case 'lifted':
      return {
        ...base,
        primary: window(100, 300, now + LIMIT_LIFTS_IN),
        secondary: window(44, 10080, now + 2 * 24 * HOUR),
        limitReached: 'rate_limit_reached',
        ordinaryUsageAllowed: false,
      };
    case 'unknown':
      return {
        ...base,
        known: false,
        reason: 'Codex CLI 0.46.0 cannot report its usage. Update it to read the usage here.',
        primary: null,
        secondary: null,
        plan: null,
        ordinaryUsageAllowed: null,
        credits: null,
        resetCredits: null,
      };
    default:
      return base;
  }
}

interface Options {
  state: string | null;
  /** The Lab's own handlers, to run the held jobs again. */
  lab: Record<string, (args: Args) => unknown>;
}

export function createCodexUsageHandlers({ state, lab }: Options) {
  let current = state;
  let last: CodexUsage | null = null;
  const read = () => (last = mockUsage(current, clock.nowMs()));

  function lift(): UsageLift {
    if (current === 'lifted') current = 'ok';
    const usage = read();
    const held = (lab.lab_get?.({ project: 'frostveil' }) as LabView).jobs.filter(
      j => j.state === 'rate-limited' && j.usageLimit
    );
    if (usage.ordinaryUsageAllowed !== true || (usage.primary?.usedPercent ?? 100) >= 100) {
      const at = usage.primary?.resetsAtMs ?? clock.nowMs();
      const time = new Date(at).toTimeString().slice(0, 5);
      return {
        usage,
        lifted: false,
        jobs: held,
        message: `The usage limit still holds until about ${time}. The jobs run again on their own then.`,
      };
    }
    const jobs = held.map(j => lab.lab_retry?.({ jobId: j.id }) as ConceptJob);
    return {
      usage,
      lifted: true,
      jobs,
      message: `The usage limit has lifted. ${jobs.length} ${jobs.length === 1 ? 'job runs' : 'jobs run'} again now.`,
    };
  }

  return {
    codex_usage_get: () => last,
    codex_usage_read: () => read(),
    lab_lift_usage_limit: () => lift(),
  };
}
