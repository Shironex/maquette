/**
 * What the Board's Sends button counts. By hand and the 3D queue read the
 * same list of provider jobs: a By hand send waiting for its model is also
 * an active send of the queue, so `inFlight` is the only count to total.
 */
import type { ProviderJob } from '@/ipc/client';
import { checklist, waitingByHand } from './batch';
import { isActive } from './providers';

export type SendsTab = 'queue' | 'hand';

export interface SendCounts {
  /** Distinct sends under way, of either kind. */
  inFlight: number;
  /** Of those, sent to a provider's API: Tripo or Meshy. */
  api: number;
  /** Of those, By hand. */
  hand: number;
  /** By hand sends waiting for you to make the model. */
  toMake: number;
  /** Paid sends waiting for your approval in the app. */
  approval: number;
  /** Sends that failed and are still listed. */
  failed: number;
}

export function sendCounts(jobs: readonly ProviderJob[]): SendCounts {
  const active = jobs.filter(j => isActive(j.state));
  const hand = active.filter(j => j.provider === 'by-hand').length;
  return {
    inFlight: active.length,
    api: active.length - hand,
    hand,
    toMake: waitingByHand(jobs).length,
    approval: active.filter(j => j.state === 'approval').length,
    failed: jobs.filter(j => j.state === 'failed').length,
  };
}

/** The list Sends opens: By hand when only By hand sends are under way, else the queue. */
export function sendsOpensOn(counts: SendCounts): SendsTab {
  return counts.hand > 0 && counts.api === 0 ? 'hand' : 'queue';
}

/** A send under way waits on you, not on a provider. */
export function sendsNeedYou(counts: SendCounts): boolean {
  return counts.toMake > 0 || counts.approval > 0;
}

/** The Sends tooltip: what is under way, by what it waits on. */
export function sendsSummary(counts: SendCounts): string {
  const parts = [
    counts.toMake > 0 ? `${counts.toMake} to make by hand` : null,
    counts.approval > 0 ? `${counts.approval} waiting for approval` : null,
    counts.failed > 0 ? `${counts.failed} failed` : null,
  ].filter(Boolean);
  const sends = counts.inFlight === 1 ? '1 send in flight' : `${counts.inFlight} sends in flight`;
  return parts.length ? `${sends}: ${parts.join(', ')}` : sends;
}

/**
 * Whether the panel shows its tabs. With one kind under way and its list
 * open there is nothing to switch to. With nothing under way, By hand is a
 * tab only when it lists something, so a tab is never empty.
 */
export function sendsShowTabs(jobs: readonly ProviderJob[], tab: SendsTab): boolean {
  const { api, hand } = sendCounts(jobs);
  if (api > 0 && hand > 0) return true;
  if (api > 0) return tab === 'hand';
  if (hand > 0) return tab === 'queue';
  return checklist(jobs).length > 0;
}
