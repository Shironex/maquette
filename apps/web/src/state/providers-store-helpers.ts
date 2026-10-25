import type { ProviderJob, SendOptions } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import type { SendDraft } from './providers';
import type { ProvidersStore } from './providers-store.types';

export const notify = (status: 'done' | 'failed' | 'info', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

export function upsert(jobs: readonly ProviderJob[], job: ProviderJob): ProviderJob[] {
  const at = jobs.findIndex(j => j.id === job.id);
  if (at < 0) return [...jobs, job];
  const next = [...jobs];
  next[at] = job;
  return next;
}

/** The queue with `job` in, when it belongs to the open project; an answer can land after a switch. */
export function foldIn(
  state: Pick<ProvidersStore, 'project' | 'jobs'>,
  job: ProviderJob
): ProviderJob[] {
  return job.project === state.project ? upsert(state.jobs, job) : state.jobs;
}

/** A draft from the saved presets, with the asset's own refs picked. */
export function draftFrom(
  options: SendOptions,
  provider: SendDraft['provider'] = 'tripo'
): SendDraft {
  const providers = useAppStore.getState().workspace?.config.providers;
  const variant = options.variant;
  const tripo = providers?.tripo[variant];
  const meshy = providers?.meshy[variant];
  if (!tripo || !meshy) throw new Error('Open a workspace first.');
  return {
    provider,
    service: provider === 'meshy' ? 'meshy' : 'tripo',
    variant,
    images: options.suggested.slice(0, 4),
    tripo: structuredClone(tripo),
    meshy: structuredClone(meshy),
  };
}

/** The saved presets, both variants, for a batch. */
export function savedPresets() {
  const providers = useAppStore.getState().workspace?.config.providers;
  if (!providers) throw new Error('Open a workspace first.');
  return { tripo: providers.tripo, meshy: providers.meshy };
}
