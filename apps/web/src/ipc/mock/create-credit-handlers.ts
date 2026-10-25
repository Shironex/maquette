/**
 * Credits for mock mode: estimates from the price lists, a ledger made of
 * the live sends plus a month of earlier ones, fake balances for the saved
 * keys and a CSV save that only says where it would have gone.
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
    'moon-island',
    'Shop house',
    'shop-house-2',
    'tripo',
    2.2,
    40,
    'succeeded',
    'shop-house-1',
    false,
  ],
  ['moon-island', 'Vending machine', 'vending-2', 'meshy', 3.5, 30, 'succeeded', null, false],
  ['rooftop-cinema', 'Vending machine', 'vending-1', 'tripo', 4.1, 30, 'succeeded', null, false],
  ['moon-island', 'Island', 'island-3', 'tripo', 5.8, 50, 'refunded', null, false],
  ['moon-island', 'Island', 'island-2', 'tripo', 6.2, 50, 'succeeded', 'island-1', false],
  ['rooftop-cinema', 'Flower shrubs', 'flower-shrubs-0', 'meshy', 9.4, 20, 'failed', null, false],
  ['moon-island', 'Island', 'island-1', 'tripo', 12, 50, 'succeeded', null, false],
  ['moon-island', 'Shop house', 'shop-house-1', 'tripo', 16, 40, 'succeeded', null, true],
  ['lantern-keeper', 'Character', 'character-1', 'tripo', 24, 70, 'succeeded', null, false],
  ['moon-island', 'Sakura', 'sakura-0', 'meshy', 41, 30, 'succeeded', null, false],
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
    viaAgent: job === 'island-2',
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
  const saved: Spend[] = history ? HISTORY.map(p => past(p, now)) : [];

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
        .map(fromJob)
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
