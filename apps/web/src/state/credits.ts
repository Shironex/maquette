/**
 * Words and numbers for the Credits view: the ledger of paid sends, the
 * week and month totals, the redo share and the CSV export.
 */
import type { Status } from '@maquette/ui';
import type { Balance, ProviderId, Spend, SpendOutcome } from '@/ipc/client';

export const DAY_MS = 86_400_000;

/** "1,240", "12.5": whole credits plain, others to one decimal. */
export function formatCredits(n: number | null | undefined): string {
  const value = n ?? 0;
  return Number.isInteger(value)
    ? value.toLocaleString('en-US')
    : value.toLocaleString('en-US', { minimumFractionDigits: 1, maximumFractionDigits: 1 });
}

/** "About 30 credits", or null when nothing is spent (By hand). */
export function estimateText(n: number | null | undefined): string | null {
  if (n === null || n === undefined) return null;
  return `About ${formatCredits(n)} ${n === 1 ? 'credit' : 'credits'}`;
}

export const OUTCOME_CHIP: Record<SpendOutcome, { status: Status; label: string }> = {
  pending: { status: 'running', label: 'Running' },
  succeeded: { status: 'done', label: 'Succeeded' },
  failed: { status: 'failed', label: 'Failed' },
  refunded: { status: 'cached', label: 'Refunded' },
};

/** A send's identity: job ids repeat across projects and once finished jobs are cleared. */
export const spendKey = (s: Spend): string => `${s.project}/${s.job}/${s.task ?? ''}`;

export interface CreditsFilter {
  project: string | null;
  service: ProviderId | null;
}

export function filterSpends(spends: readonly Spend[], filter: CreditsFilter): Spend[] {
  return spends.filter(
    s =>
      (!filter.project || s.project === filter.project) &&
      (!filter.service || s.service === filter.service)
  );
}

export interface CreditTotals {
  /** Spent in the last 7 days. */
  week: number;
  /** Spent in the last 30 days. */
  month: number;
  total: number;
  sends: number;
  redos: number;
  /** Credits that went on redos, over all credits, 0 to 1. */
  redoShare: number;
  /** Sends whose credits are the price list's guess. */
  estimated: number;
}

/** The totals of `spends` as of `now`; the windows count back from it. */
export function creditTotals(spends: readonly Spend[], now: number): CreditTotals {
  let week = 0;
  let month = 0;
  let total = 0;
  let redoCredits = 0;
  let redos = 0;
  let estimated = 0;
  for (const s of spends) {
    const credits = s.credits ?? 0;
    const age = now - (s.sentAtMs ?? 0);
    total += credits;
    if (age <= 7 * DAY_MS) week += credits;
    if (age <= 30 * DAY_MS) month += credits;
    if (s.redoOf) {
      redos += 1;
      redoCredits += credits;
    }
    if (s.estimated) estimated += 1;
  }
  return {
    week,
    month,
    total,
    sends: spends.length,
    redos,
    redoShare: total > 0 ? redoCredits / total : 0,
    estimated,
  };
}

/** "18%" of credits on redos. */
export function shareText(share: number): string {
  return `${Math.round(share * 100)}%`;
}

/** The projects the ledger mentions, in the order they first appear. */
export function ledgerProjects(spends: readonly Spend[]): string[] {
  return [...new Set(spends.map(s => s.project))];
}

const CSV_HEADER = [
  'sent_at',
  'project',
  'asset',
  'job',
  'provider',
  'task',
  'settings',
  'credits',
  'estimated',
  'outcome',
  'redo_of',
  'via_agent',
];

/** A cell quoted when it holds a comma, a quote or a line break. */
function cell(value: string | number | boolean | null | undefined): string {
  const text = value === null || value === undefined ? '' : String(value);
  return /[",\r\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

/** The spends as CSV, one row per send, oldest first. */
export function spendsCsv(spends: readonly Spend[]): string {
  const rows = [...spends]
    .sort((a, b) => (a.sentAtMs ?? 0) - (b.sentAtMs ?? 0))
    .map(s =>
      [
        s.sentAtMs ? new Date(s.sentAtMs).toISOString() : '',
        s.project,
        s.assetName,
        s.job,
        s.service,
        s.task,
        s.summary,
        s.credits ?? 0,
        s.estimated,
        s.outcome,
        s.redoOf,
        s.viaAgent,
      ]
        .map(cell)
        .join(',')
    );
  return [CSV_HEADER.join(','), ...rows].join('\n') + '\n';
}

/** "maquette-credits-2026-10-25.csv". */
export function csvName(now: number): string {
  return `maquette-credits-${new Date(now).toISOString().slice(0, 10)}.csv`;
}

export interface BalanceView {
  value: string;
  note: string;
  tone: 'ok' | 'mute' | 'error';
}

/** A balance card's number and line: credits left, no key, or why the ask failed. */
export function balanceView(
  balance: Pick<Balance, 'key' | 'credits' | 'held' | 'error'> | undefined,
  checking: boolean
): BalanceView {
  if (!balance) return { value: '...', note: checking ? 'Asking' : 'Not asked yet', tone: 'mute' };
  if (balance.error) return { value: 'Unknown', note: balance.error, tone: 'error' };
  if (!balance.key)
    return { value: 'No key', note: 'Add one in Settings > 3D providers', tone: 'mute' };
  const held = balance.held ? `${formatCredits(balance.held)} held by running tasks` : null;
  return {
    value: formatCredits(balance.credits),
    note: held ?? 'credits left',
    tone: 'ok',
  };
}
