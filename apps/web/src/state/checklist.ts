/** The By hand checklist: sends grouped by batch, and which models are back. */
import type { ProviderJob } from '@/ipc/client';

export interface ChecklistGroup {
  /** The batch id, or null for single sends. */
  batch: string | null;
  jobs: ProviderJob[];
  /** How many models came back. */
  back: number;
  createdAtMs: number;
}

const BACK: ReadonlySet<ProviderJob['state']> = new Set(['inbox', 'accepted']);

/** The sends of a group that were not stopped: what the batch really holds. */
export function liveJobs(group: Pick<ChecklistGroup, 'jobs'>): ProviderJob[] {
  return group.jobs.filter(j => j.state !== 'cancelled');
}

/**
 * Every By hand send, grouped by batch. A group with a model still waiting
 * comes first, so the work to do is not buried under finished batches; each
 * half is newest first. A group shows while any of its sends was not
 * cancelled, and inside it the waiting sends come first.
 */
export function checklist(jobs: readonly ProviderJob[]): ChecklistGroup[] {
  const groups = new Map<string, ChecklistGroup>();
  for (const job of jobs) {
    if (job.provider !== 'by-hand') continue;
    const key = job.batch ?? `single:${job.id}`;
    const group = groups.get(key) ?? {
      batch: job.batch,
      jobs: [],
      back: 0,
      createdAtMs: job.createdAtMs ?? 0,
    };
    group.jobs.push(job);
    if (BACK.has(job.state)) group.back += 1;
    group.createdAtMs = Math.max(group.createdAtMs, job.createdAtMs ?? 0);
    groups.set(key, group);
  }
  return [...groups.values()]
    .filter(g => g.jobs.some(j => j.state !== 'cancelled'))
    .map(g => ({
      ...g,
      jobs: [...g.jobs].sort(
        (a, b) =>
          Number(wantsModel(b)) - Number(wantsModel(a)) || a.assetName.localeCompare(b.assetName)
      ),
    }))
    .sort(
      (a, b) =>
        Number(a.jobs.some(wantsModel) ? 0 : 1) - Number(b.jobs.some(wantsModel) ? 0 : 1) ||
        b.createdAtMs - a.createdAtMs
    );
}

/** True when the job still wants a model: the checklist row offers Add model. */
export function wantsModel(job: ProviderJob): boolean {
  return job.provider === 'by-hand' && job.state === 'waiting';
}

/** By hand sends still waiting for a model. */
export function waitingByHand(jobs: readonly ProviderJob[]): ProviderJob[] {
  return jobs.filter(wantsModel);
}

/** "4 of 6 back", "All 6 back". */
export function backLabel(group: Pick<ChecklistGroup, 'jobs' | 'back'>): string {
  const live = liveJobs(group).length;
  return group.back >= live ? `All ${live} back` : `${group.back} of ${live} back`;
}
