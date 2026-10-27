/**
 * Credits for mock mode: estimates from the price lists, a ledger made of
 * the live sends plus a month of earlier ones (API spend, and By hand sends
 * paid from a subscription), fake balances for the saved keys and a CSV
 * save that only says where it would have gone.
 */
import type {
  Balance,
  BatchRequest,
  Ledger,
  ProviderId,
  ProviderJob,
  SendRequest,
  Spend,
  SpendOutcome,
  WorkspaceConfig,
} from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';
import { estimate } from './pricing';

const DAY = 86_400_000;

const BALANCE: Record<ProviderId, { credits: number; held: number | null }> = {
  tripo: { credits: 1240, held: 30 },
  meshy: { credits: 380, held: null },
};

/** Earlier paid sends: [project, asset name, job, service, days ago, credits, outcome, redo of, estimated]. */
type Past = [
  string,
  string,
  string,
  ProviderId,
  number,
  number,
  SpendOutcome,
  string | null,
  boolean,
];
const HISTORY: readonly Past[] = [
  [
    'frostveil',
    'Observatory arch',
    'observatory-arch-2',
    'tripo',
    2.2,
    40,
    'succeeded',
    'observatory-arch-1',
    false,
  ],
  ['frostveil', 'Crystal lantern', 'crystal-lantern-2', 'meshy', 3.5, 30, 'succeeded', null, false],
  [
    'frost-cathedral',
    'Crystal lantern',
    'crystal-lantern-1',
    'tripo',
    4.1,
    30,
    'succeeded',
    null,
    false,
  ],
  ['frostveil', 'Veil platform', 'veil-platform-3', 'tripo', 5.8, 50, 'refunded', null, false],
  [
    'frostveil',
    'Veil platform',
    'veil-platform-2',
    'tripo',
    6.2,
    50,
    'succeeded',
    'veil-platform-1',
    false,
  ],
  ['frost-cathedral', 'Ice crystals', 'ice-crystals-0', 'meshy', 9.4, 20, 'failed', null, false],
  ['frostveil', 'Veil platform', 'veil-platform-1', 'tripo', 12, 50, 'succeeded', null, false],
  ['frostveil', 'Observatory arch', 'observatory-arch-1', 'tripo', 16, 40, 'succeeded', null, true],
  ['elsheria-showcase', 'Elsheria', 'elsheria-1', 'tripo', 24, 70, 'succeeded', null, false],
  ['frostveil', 'Veil gate', 'veil-gate-0', 'meshy', 41, 30, 'succeeded', null, false],
];

const SUMMARY: Record<ProviderId, string> = {
  tripo: 'Tripo H3.1, 1 image, 30k faces',
  meshy: 'Meshy 6, 1 image, 2K PBR',
};

function past(
  [project, assetName, job, service, days, credits, outcome, redoOf, estimated]: Past,
  now: number
): Spend {
  const sent = now - days * DAY;
  return {
    project,
    job,
    asset: job.replace(/-\d+$/, ''),
    assetName,
    service,
    task: `${job}-task`,
    summary: SUMMARY[service],
    sentAtMs: sent,
    settledAtMs: sent + 3 * 60_000,
    credits: outcome === 'succeeded' ? credits : 0,
    estimated,
    outcome,
    redoOf,
    viaAgent: job === 'veil-platform-2',
    source: 'api',
  };
}

/** Earlier By hand sends: [asset name, job, service, days ago, credits, what]. */
const BY_HAND: readonly [string, string, ProviderId, number, number, string][] = [
  ['Rune lantern', 'rune-lantern-1', 'meshy', 1.5, 20, 'mesh only'],
  ['Observatory bench', 'observatory-bench-2', 'tripo', 4.5, 30, 'textured image to 3D'],
  ['Observatory bench', 'observatory-bench-1', 'tripo', 8, 30, 'textured image to 3D'],
];

function pastByHand(
  [assetName, job, service, days, credits, what]: (typeof BY_HAND)[number],
  now: number
): Spend {
  const sent = now - days * DAY;
  return {
    project: 'frostveil',
    job,
    asset: job.replace(/-\d+$/, ''),
    assetName,
    service,
    task: null,
    summary: `By hand on the ${service === 'tripo' ? 'Tripo' : 'Meshy'} site, ${what}`,
    sentAtMs: sent,
    settledAtMs: sent + 40 * 60_000,
    credits,
    estimated: true,
    outcome: 'succeeded',
    redoOf: job === 'observatory-bench-2' ? 'observatory-bench-1' : null,
    viaAgent: false,
    source: 'subscription',
  };
}

const BY_HAND_OUTCOME: Partial<Record<ProviderJob['state'], SpendOutcome>> = {
  waiting: 'pending',
  inbox: 'succeeded',
  accepted: 'succeeded',
  cancelled: 'cancelled',
};

/** A live By hand send as the ledger lists it: the site's estimate, settled on import. */
function fromHandoff(job: ProviderJob): Spend | null {
  const outcome = BY_HAND_OUTCOME[job.state];
  const estimate = job.card?.credits;
  if (job.provider !== 'by-hand' || !outcome || estimate === null || estimate === undefined)
    return null;
  const sent = job.createdAtMs ?? clock.nowMs();
  return {
    project: job.project,
    job: job.id,
    asset: job.asset,
    assetName: job.assetName,
    service: job.service,
    task: null,
    summary: `By hand on the ${job.service === 'tripo' ? 'Tripo' : 'Meshy'} site, ${job.card?.creditsNote ?? 'one model'}`,
    sentAtMs: sent,
    settledAtMs: outcome === 'pending' ? null : (job.finishedAtMs ?? sent),
    credits: outcome === 'cancelled' ? 0 : estimate,
    estimated: outcome !== 'cancelled',
    outcome,
    redoOf: job.redoOf ?? null,
    viaAgent: job.viaAgent,
    source: 'subscription',
  };
}

const OUTCOME: Partial<Record<ProviderJob['state'], SpendOutcome>> = {
  running: 'pending',
  inbox: 'succeeded',
  accepted: 'succeeded',
  failed: 'failed',
  cancelled: 'refunded',
};

/** A live send as the ledger lists it, once a provider task exists for it. */
function fromJob(job: ProviderJob): Spend | null {
  const outcome = OUTCOME[job.state];
  if (job.provider === 'by-hand' || !job.startedAtMs || !outcome) return null;
  const reported = job.credits ?? null;
  const settled = outcome !== 'pending';
  const credits =
    outcome === 'refunded'
      ? 0
      : outcome === 'failed'
        ? (reported ?? 0)
        : (reported ?? job.estimate ?? 0);
  return {
    project: job.project,
    job: job.id,
    asset: job.asset,
    assetName: job.assetName,
    service: job.service,
    task: job.remoteId ?? `${job.id}-task`,
    summary: SUMMARY[job.service],
    sentAtMs: job.startedAtMs,
    settledAtMs: settled ? job.finishedAtMs : null,
    credits,
    estimated: outcome !== 'refunded' && outcome !== 'failed' && reported === null,
    outcome,
    redoOf: job.redoOf ?? null,
    viaAgent: job.viaAgent,
    source: 'api',
  };
}

export function createCreditHandlers({
  config,
  jobs,
  hasKey,
  history,
}: {
  config: () => WorkspaceConfig;
  jobs: () => ProviderJob[];
  hasKey: (service: ProviderId) => boolean;
  /** The workspace has a month of earlier sends. */
  history: boolean;
}): Record<string, (args: Args) => unknown> {
  const now = clock.nowMs();
  const saved: Spend[] = history
    ? [...HISTORY.map(p => past(p, now)), ...BY_HAND.map(p => pastByHand(p, now))]
    : [];

  const sendEstimate = (req: SendRequest): number | null => {
    const presets = config().providers;
    return estimate(
      req.provider,
      req.variant,
      req.tripo ?? presets.tripo[req.variant],
      req.meshy ?? presets.meshy[req.variant]
    );
  };

  return {
    provider_estimate: ({ request }) => sendEstimate(request as SendRequest),
    provider_estimate_batch: ({ request }) => {
      const req = request as BatchRequest;
      if (req.provider === 'by-hand') return null;
      return req.items.reduce((sum, item) => {
        const preset = <T>(own: T | null, character: T | null, prop: T | null) =>
          own ?? (item.variant === 'character' ? character : prop);
        const each = sendEstimate({
          project: req.project,
          asset: item.asset,
          provider: req.provider,
          service: req.service,
          variant: item.variant,
          images: item.images,
          tripo: preset(item.tripo, req.tripoCharacter, req.tripoProp),
          meshy: preset(item.meshy, req.meshyCharacter, req.meshyProp),
        });
        return sum + (each ?? 0);
      }, 0);
    },
    credits_ledger: (): Ledger => {
      const live = jobs()
        .map(j => fromJob(j) ?? fromHandoff(j))
        .filter((s): s is Spend => s !== null);
      const spends = [...live, ...saved].sort((a, b) => (b.sentAtMs ?? 0) - (a.sentAtMs ?? 0));
      return { spends, skipped: history ? 1 : 0 };
    },
    credits_balances: (): Balance[] =>
      (['tripo', 'meshy'] as const).map(service => ({
        service,
        key: hasKey(service),
        credits: hasKey(service) ? BALANCE[service].credits : null,
        held: hasKey(service) ? BALANCE[service].held : null,
        error: null,
        checkedAtMs: clock.nowMs(),
      })),
    credits_save_csv: ({ name }) => `~/Downloads/${String(name)}`,
  };
}
