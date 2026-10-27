/**
 * Remove from project: which sends a remove stops or waits for, and what
 * the notice says afterwards. The core makes the same calls; this only
 * tells the person first.
 */
import type { Asset, ProviderJob, RemoveOutcome } from '@/ipc/client';
import { PROVIDER_LABEL } from './providers';
import type { Notice } from './use-app-store';

/** The most assets one remove takes; the core says the same. */
export const MAX_REMOVE = 50;

const STOPS: readonly ProviderJob['state'][] = ['approval', 'queued', 'waiting'];

export interface RemovePlan {
  /** Sends waiting for a model, which the remove stops. */
  stops: ProviderJob[];
  /** Paid sends being made now: their assets are refused until they are over. */
  running: ProviderJob[];
}

export function removePlan(assets: readonly Asset[], jobs: readonly ProviderJob[]): RemovePlan {
  const ids = new Set(assets.map(a => a.id));
  const mine = jobs.filter(j => ids.has(j.asset));
  return {
    stops: mine.filter(j => STOPS.includes(j.state)),
    running: mine.filter(j => j.state === 'running'),
  };
}

/** "Clay bowl", or "10 assets". */
export function assetsLabel(assets: readonly { name: string }[]): string {
  return assets.length === 1 ? (assets[0]?.name ?? '1 asset') : `${assets.length} assets`;
}

/** What a stopped send was waiting for, in a few words. */
export function stopLabel(job: ProviderJob): string {
  if (job.state === 'waiting') return 'waiting for your model, made by hand';
  if (job.state === 'approval')
    return `waiting for approval to send to ${PROVIDER_LABEL[job.provider]}`;
  return `queued for ${PROVIDER_LABEL[job.provider]}, nothing sent yet`;
}

/** The notice after a remove, without its Undo. */
export function removedNotice(outcomes: readonly RemoveOutcome[]): Omit<Notice, 'id'> {
  const removed = outcomes.filter(o => o.state === 'removed');
  const refused = outcomes.filter(o => o.state === 'refused');
  const missing = outcomes.filter(o => o.state === 'not-found');
  const named = (list: readonly RemoveOutcome[]) =>
    assetsLabel(list.map(o => ({ name: o.name ?? o.asset })));
  const why = refused.map(o => o.reason ?? `${o.name ?? o.asset} was refused.`).join(' ');
  const gone =
    missing.length === 0
      ? ''
      : ` ${missing.length === 1 ? '1 was' : `${missing.length} were`} already gone.`;
  if (removed.length === 0) {
    return refused.length > 0
      ? { status: 'failed', title: `Could not remove ${named(refused)}`, detail: why + gone }
      : { status: 'info', title: 'Nothing to remove', detail: gone.trim() || undefined };
  }
  const where =
    removed.length === 1
      ? 'Its files are in the workspace trash; Undo or Recently removed puts it back.'
      : 'Their files are in the workspace trash; Undo or Recently removed puts them back.';
  if (refused.length === 0) {
    return { status: 'done', title: `Removed ${named(removed)}`, detail: where + gone };
  }
  return {
    status: 'info',
    title: `Removed ${removed.length} of ${outcomes.length} assets`,
    detail: `${why}${gone} ${where}`,
  };
}

/** The notice after a restore: the names that came back and why others did not. */
export function restoredNotice(
  back: readonly string[],
  failed: readonly string[]
): Omit<Notice, 'id'> {
  const detail =
    'Back on the Board with its files, place and settings. Stopped sends stay stopped.';
  if (failed.length === 0) {
    return {
      status: 'done',
      title: back.length === 1 ? `Restored ${back[0]}` : `Restored ${back.length} assets`,
      detail,
    };
  }
  if (back.length === 0) {
    return {
      status: 'failed',
      title:
        failed.length === 1 ? 'Could not restore it' : `Could not restore ${failed.length} assets`,
      detail: failed.join(' '),
    };
  }
  return {
    status: 'info',
    title: `Restored ${back.length} of ${back.length + failed.length} assets`,
    detail: failed.join(' '),
  };
}
