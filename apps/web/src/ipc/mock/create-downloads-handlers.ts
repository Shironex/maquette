/**
 * The Downloads watcher for mock mode: on, watching the fake Downloads
 * folder, with the waiting By hand sends as its pending hand-offs.
 * `?arrival=named` drops an export named for the first waiting send a
 * moment after the app opens (it is copied into the inbox), and
 * `?arrival=unmatched` drops "model (3).glb", which asks which asset it is.
 */
import type { Arrival, PendingHandoff, ProviderJob, WatchStatus } from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { HOME } from './fixtures';
import { exportName } from './handoff-card';

const DOWNLOADS = `${HOME}/Downloads`;
const ARRIVAL_MS = 1200;

function pendingOf(job: ProviderJob): PendingHandoff {
  return {
    job: job.id,
    project: job.project,
    asset: job.asset,
    assetName: job.assetName,
    service: job.service,
    expectedName: job.card?.exportName || exportName(job),
    handoff: job.handoff,
    credits: job.card?.credits ?? null,
    createdAtMs: job.createdAtMs,
  };
}

export function createDownloadsHandlers({
  jobs,
  handBack,
  emit,
  arrival = null,
  delayMs = ARRIVAL_MS,
}: {
  jobs: () => ProviderJob[];
  /** The provider mock's hand-back: copies a file into the inbox for a send. */
  handBack: (args: Args) => unknown;
  emit: (arrival: Arrival) => void;
  arrival?: string | null;
  /** How long after opening the download lands. */
  delayMs?: number;
}): Record<string, (args: Args) => unknown> {
  let enabled = true;
  let custom: string | null = null;
  let arrivals: Arrival[] = [];
  const waiting = () => jobs().filter(j => j.provider === 'by-hand' && j.state === 'waiting');

  const status = (): WatchStatus => ({
    enabled,
    folder: custom ?? DOWNLOADS,
    custom: custom !== null,
    watching: enabled,
    error: null,
    pending: waiting().map(pendingOf),
    arrivals,
  });

  const remember = (next: Arrival): Arrival => {
    arrivals = [next, ...arrivals.filter(a => a.id !== next.id)];
    emit(next);
    return next;
  };

  const file = (name: string, n: number): Arrival => ({
    id: `a1b2c3d4e5f6000${n}`,
    path: `${DOWNLOADS}/${name}`,
    name,
    inner: null,
    bytes: 4.2e6,
    atMs: clock.nowMs(),
    state: 'unmatched',
    job: null,
    project: null,
    assetName: null,
    reason: '',
    inboxFile: null,
  });

  const take = (base: Arrival, job: ProviderJob, reason: string): Arrival => {
    handBack({ id: job.id, file: base.path });
    return remember({
      ...base,
      state: 'imported',
      job: job.id,
      project: job.project,
      assetName: job.assetName,
      reason,
      inboxFile: `${job.id}.glb`,
    });
  };

  if (arrival === 'named' || arrival === 'unmatched') {
    setTimeout(() => {
      const [first] = waiting();
      if (!first) return;
      if (arrival === 'named') {
        take(file(`${exportName(first)}.glb`, 1), first, `Named for ${first.assetName}`);
      } else {
        remember({ ...file('model (3).glb', 2), reason: `${waiting().length} sends are waiting` });
      }
    }, delayMs);
  }

  const find = (id: unknown): Arrival =>
    arrivals.find(a => a.id === id) ?? fail('not-found', `Could not find the download ${id}.`);

  return {
    downloads_watch_status: status,
    downloads_watch_set: ({ enabled: on, folder: dir }) => {
      if (typeof dir === 'string' && /missing/i.test(dir))
        fail('invalid', `${dir} is not a folder.`);
      enabled = Boolean(on);
      custom = typeof dir === 'string' && dir.trim() ? dir.trim() : null;
      return status();
    },
    downloads_assign: ({ arrival: id, job: jobId }) => {
      const found = find(id);
      if (found.state !== 'unmatched' && found.state !== 'failed')
        fail('invalid', `${found.name} was already taken care of.`);
      const job =
        waiting().find(j => j.id === jobId) ??
        fail('invalid', `The send ${String(jobId)} is not waiting for a model.`);
      return take(found, job, `You chose ${job.assetName}`);
    },
    downloads_dismiss: ({ arrival: id }) => {
      const next: Arrival = { ...find(id), state: 'dismissed', reason: 'Not a model for Maquette' };
      arrivals = arrivals.map(a => (a.id === next.id ? next : a));
      return next;
    },
  };
}
