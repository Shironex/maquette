import type { Asset, ProjectManifest, ProviderJob } from '../client';

const ON_ITS_WAY: readonly string[] = ['approval', 'queued', 'running', 'waiting', 'inbox'];

/**
 * The project after send `job` was denied: its card goes back from the 3D
 * model column to Refs while it has no model and no other send of it is on
 * its way or in the Inbox, as the core does.
 */
export function deniedCardBack(
  p: ProjectManifest,
  job: ProviderJob,
  jobs: readonly ProviderJob[]
): ProjectManifest {
  const others = jobs.some(
    j =>
      j.id !== job.id &&
      j.project === job.project &&
      j.asset === job.asset &&
      ON_ITS_WAY.includes(j.state)
  );
  const back = (a: Asset): Asset =>
    a.id === job.asset && a.stage === 'tripo' && (a.versions ?? []).length === 0 && !others
      ? { ...a, stage: 'refs', status: 'queued' }
      : a;
  return { ...p, assets: (p.assets ?? []).map(back) };
}

/** Send `job` as a denial leaves it: cancelled, nothing spent, marked denied. */
export const deniedJob = (job: ProviderJob, at: number): ProviderJob => ({
  ...job,
  state: 'cancelled',
  finishedAtMs: at,
  message: 'Denied in Maquette. Nothing was spent.',
  cancel: 'none',
  denied: true,
});
