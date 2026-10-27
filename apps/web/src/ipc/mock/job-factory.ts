import type { ProviderId, ProviderJob, WorkspaceConfig } from '../client';
import { estimate } from './pricing';

export type JobPartial = Partial<ProviderJob> &
  Pick<ProviderJob, 'id' | 'asset' | 'assetName' | 'provider'>;

/** Makes fake jobs: the saved prop preset, queued, with the price list's estimate. */
export function jobFactory(config: () => WorkspaceConfig, now: number) {
  return (partial: JobPartial): ProviderJob => {
    const presets = config().providers;
    const service: ProviderId = partial.provider === 'meshy' ? 'meshy' : 'tripo';
    const job: ProviderJob = {
      project: 'frostveil',
      service,
      variant: 'prop',
      images: [],
      tripo: service === 'tripo' ? structuredClone(presets.tripo.prop) : null,
      meshy: service === 'meshy' ? structuredClone(presets.meshy.prop) : null,
      state: 'queued',
      phase: null,
      progress: 0,
      remoteId: null,
      credits: null,
      createdAtMs: now,
      startedAtMs: null,
      finishedAtMs: null,
      message: null,
      inboxFile: null,
      handoff: null,
      card: null,
      version: null,
      viaAgent: false,
      cancel: 'free',
      batch: null,
      prompt: null,
      redoOf: null,
      ...partial,
    };
    return {
      ...job,
      estimate: job.estimate ?? estimate(job.provider, job.variant, job.tripo, job.meshy),
    };
  };
}
