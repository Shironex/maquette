import { useState } from 'react';
import { commands, errorMessage, type ProviderJob } from '@/ipc/client';
import { notify, upsert } from '@/state/providers-store-helpers';
import { MESHY_RETEXTURE_CREDITS, bakeJobs, retextureProblem, sourceLabel } from '@/state/split';
import { estimateText } from '@/state/credits';
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
  const [asking, setAsking] = useState(false);
  const [prompt, setPrompt] = useState('');
  const [sending, setSending] = useState(false);
  // A re-texture of the same version answers the offer, unless it came to nothing.
  const painting = bake
    ? jobs.find(
        j =>
          j.project === project &&
          j.asset === asset &&
          j.retexture?.fromVersion === bake.onto &&
          !['failed', 'cancelled'].includes(j.state)
      )
    : undefined;

  const send = async () => {
    if (!bake || !split) return;
    setSending(true);
    try {
      const next = await commands.retextureSend({
        project,
        asset,
        version: bake.onto,
        prompt,
      });
      useProvidersStore.setState(s => ({ jobs: upsert(s.jobs, next) }));
      setAsking(false);
      notify(
        'done',
        `${next.assetName} is on its way to Meshy`,
        'It lands in the Inbox as the next version; the grey one stays.'
      );
    } catch (e) {
      notify('failed', 'Could not send the re-texture', errorMessage(e));
    } finally {
      setSending(false);
    }
  };

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
    meshy: job?.provider === 'meshy',
    painting,
    /** The question before Meshy's credits go out. */
    retexture: {
      open: asking,
      ask: () => {
        setPrompt(split?.texturePrompt ?? '');
        setAsking(true);
      },
      keep: () => setAsking(false),
      prompt,
      setPrompt,
      problem: retextureProblem(prompt),
      cost: estimateText(MESHY_RETEXTURE_CREDITS),
      sending,
      send: () => void send(),
    },
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
