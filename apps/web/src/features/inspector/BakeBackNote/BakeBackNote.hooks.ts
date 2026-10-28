import { useState } from 'react';
import { commands, errorMessage, type ProviderJob } from '@/ipc/client';
import { notify, upsert } from '@/state/providers-store-helpers';
import { bakeJobs, sourceLabel } from '@/state/split';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The asset's newest split with a bake back, and what can be done with it. */
export function useBakeBackNote(project: string, asset: string) {
  const jobs = useProvidersStore(s => s.jobs);
  const setProject = useAppStore(s => s.setProject);
  const [busy, setBusy] = useState(false);
  const job = bakeJobs(jobs, project, asset)[0];
  const split = job?.split;
  const bake = split?.bake ?? null;

  const run = async (act: () => Promise<ProviderJob | null>) => {
    setBusy(true);
    try {
      const next = await act();
      if (next) useProvidersStore.setState(s => ({ jobs: upsert(s.jobs, next) }));
    } catch (e) {
      notify('failed', 'Could not bake the texture back', errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return {
    bake,
    from: split ? sourceLabel({ version: split.fromVersion, item: split.fromItem }) : '',
    busy,
    bakeNow: () => job && void run(() => commands.splitBakeBack(job.id)),
    keepGrey: () =>
      bake &&
      void run(async () => {
        setProject(await commands.partsKeepVersion(project, asset, bake.onto));
        return null;
      }),
  };
}
