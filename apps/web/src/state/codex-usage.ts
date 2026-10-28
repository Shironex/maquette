/**
 * Codex usage as the status bar and the Lab show it: the windows in a few
 * words, the tone, the reset times in local time, the warning before a
 * large round and what a waiting job's time comes from. Mirrors
 * `CodexUsage::limit_active` and `resets_at_ms` in the core.
 */
import type { CodexUsage, ConceptJob, UsageWindow } from '@/ipc/client';
import { formatClock } from '@/lib/format';

/** Amber from here. */
export const USAGE_WARN_PERCENT = 80;
/** A round of this many images or more counts as large. */
export const LARGE_ROUND = 4;

export type UsageTone = 'ok' | 'warn' | 'limit';

const used = (w: UsageWindow | null | undefined): number | null =>
  w && w.usedPercent !== null ? Math.max(0, Math.min(100, w.usedPercent)) : null;

const full = (w: UsageWindow | null | undefined) => (used(w) ?? 0) >= 100;

/** "5h" for the five hour window, "week" for the weekly one. */
export function windowName(w: UsageWindow): string {
  const mins = w.windowMins;
  if (mins === null) return 'window';
  if (mins === 10080) return 'week';
  if (mins % 1440 === 0) return `${mins / 1440}d`;
  if (mins % 60 === 0) return `${mins / 60}h`;
  return `${mins}m`;
}

function windows(usage: CodexUsage): UsageWindow[] {
  return [usage.primary, usage.secondary].filter((w): w is UsageWindow => used(w) !== null);
}

/** True while the usage limit holds by this read. */
export function limitActive(usage: CodexUsage | null): boolean {
  if (!usage?.known) return false;
  return (
    usage.limitReached !== null ||
    usage.ordinaryUsageAllowed === false ||
    windows(usage).some(w => full(w))
  );
}

/** When the limit that holds lifts: the latest reset of a full window, else the five hour one. */
export function limitResetsAt(usage: CodexUsage | null): number | null {
  if (!usage || !limitActive(usage)) return null;
  const resets = windows(usage)
    .filter(w => full(w) && w.resetsAtMs !== null)
    .map(w => w.resetsAtMs as number);
  if (resets.length) return Math.max(...resets);
  return usage.primary?.resetsAtMs ?? null;
}

/** The fullest window's percent, or null when none is known. */
export function mostUsed(usage: CodexUsage | null): number | null {
  if (!usage?.known) return null;
  const all = windows(usage).map(w => used(w) as number);
  return all.length ? Math.max(...all) : null;
}

export function usageTone(usage: CodexUsage | null): UsageTone {
  if (limitActive(usage)) return 'limit';
  return (mostUsed(usage) ?? 0) >= USAGE_WARN_PERCENT ? 'warn' : 'ok';
}

/** "Codex 16% (5h) · 3% (week)", or "Codex usage unknown". */
export function usageLabel(usage: CodexUsage): string {
  const parts = usage.known
    ? windows(usage).map(w => `${Math.round(used(w) as number)}% (${windowName(w)})`)
    : [];
  return parts.length ? `Codex ${parts.join(' · ')}` : 'Codex usage unknown';
}

/** "Thu 16:20" within the week, "16:20" today. */
export function resetTime(ms: number, now: number): string {
  const at = new Date(ms);
  const today = new Date(now);
  if (at.toDateString() === today.toDateString()) return formatClock(ms);
  return `${at.toLocaleDateString('en-GB', { weekday: 'short' })} ${formatClock(ms)}`;
}

/** The tooltip lines: each window's reset in local time, and why when unknown. */
export function usageDetail(usage: CodexUsage, now: number): string {
  if (!usage.known) return usage.reason ?? 'Codex did not say how much usage is left.';
  const lines = windows(usage).map(w => {
    const name = windowName(w) === 'week' ? 'Weekly' : `${windowName(w)} window`;
    const reset = w.resetsAtMs !== null ? `, resets ${resetTime(w.resetsAtMs, now)}` : '';
    return `${name}: ${Math.round(used(w) as number)}% used${reset}`;
  });
  if (limitActive(usage)) {
    const at = limitResetsAt(usage);
    lines.unshift(
      at !== null ? `Usage limit reached until about ${resetTime(at, now)}` : 'Usage limit reached'
    );
  }
  return lines.join('. ');
}

/**
 * The line shown before a large round when little is left: a round of
 * `count` images while a window is at 80% or more, or any round while the
 * limit holds. Null when there is nothing to say.
 */
export function roundWarning(usage: CodexUsage | null, count: number, now: number): string | null {
  if (!usage?.known) return null;
  if (limitActive(usage)) {
    const at = limitResetsAt(usage);
    return at !== null
      ? `Codex's usage limit holds until about ${resetTime(at, now)}. These images wait in the queue until then.`
      : "Codex's usage limit holds. These images wait in the queue until it lifts.";
  }
  if (count < LARGE_ROUND) return null;
  const low = windows(usage)
    .filter(w => (used(w) as number) >= USAGE_WARN_PERCENT)
    .sort((a, b) => (used(b) as number) - (used(a) as number))[0];
  if (!low) return null;
  const left = 100 - Math.round(used(low) as number);
  const reset = low.resetsAtMs !== null ? `, which resets ${resetTime(low.resetsAtMs, now)}` : '';
  return `Only ${left}% of Codex's ${windowName(low) === 'week' ? 'weekly' : windowName(low)} usage is left${reset}. A round of ${count} may stop at the usage limit and wait.`;
}

export interface HeldBanner {
  title: string;
  detail: string;
}

/** True for a job waiting for the usage limit to lift. */
export const usageHeld = (job: Pick<ConceptJob, 'state' | 'usageLimit'>) =>
  job.state === 'rate-limited' && job.usageLimit;

/**
 * The Lab's banner while the usage limit holds jobs: the reset time from
 * the usage read when it has one, else the time the jobs wait until (read
 * from Codex's message, or the backoff's guess). Null when none waits.
 */
export function heldBanner(
  jobs: readonly ConceptJob[],
  usage: CodexUsage | null,
  now: number
): HeldBanner | null {
  const held = jobs.filter(usageHeld);
  if (!held.length) return null;
  const n = held.length;
  const images = `${n} ${n === 1 ? 'image waits' : 'images wait'}`;
  const run = n === 1 ? 'runs' : 'run';
  const reset = limitResetsAt(usage);
  if (reset !== null) {
    return {
      title: 'Codex usage limit reached',
      detail: `Codex says it lifts at about ${resetTime(reset, now)}. ${images} and ${run} again on their own then.`,
    };
  }
  const times = held.map(j => j.retryAtMs).filter((t): t is number => t !== null);
  const at = times.length ? Math.max(...times) : null;
  return {
    title: 'Codex usage limit reached',
    detail:
      at !== null
        ? `${images} and ${run} again on their own at about ${resetTime(at, now)}.`
        : `${images} until it lifts.`,
  };
}
