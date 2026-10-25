import type { StateCreator } from 'zustand';
import { commands, errorMessage } from '@/ipc/client';
import { batchDraft } from './batch';
import { draftFrom, savedPresets } from './providers-store-helpers';
import type { ProvidersStore } from './providers-store.types';
import { redoBatchDraft, redoDraft, type RedoSources } from './redo';

/** Redo from the Inbox: the send panel and the batch, prefilled from earlier sends. */
export type RedoSlice = Pick<ProvidersStore, 'openRedo' | 'openBatchRedo' | 'setItemPrompt'>;

const GONE = 'The send behind this model is no longer on record, so there is nothing to repeat.';

export const createRedoSlice: StateCreator<ProvidersStore, [], [], RedoSlice> = (set, get) => ({
  openRedo: async (project, asset, of) => {
    set({
      batch: null,
      panel: {
        project,
        asset,
        options: null,
        draft: null,
        initial: null,
        handoff: null,
        error: null,
        redo: null,
      },
    });
    try {
      const options = await commands.providerSendOptions(project, asset);
      const current = get().panel;
      if (current?.asset !== asset) return;
      const job = options.jobs.find(j => j.id === of);
      if (!job) {
        set({ keys: options.keys, panel: { ...current, error: GONE } });
        return;
      }
      const fresh = draftFrom({ ...options, variant: job.variant }, job.provider);
      const draft = redoDraft(job, options, fresh);
      set({ keys: options.keys, panel: { ...current, options, draft, initial: draft, redo: job } });
    } catch (error) {
      const current = get().panel;
      if (current?.asset === asset) set({ panel: { ...current, error: errorMessage(error) } });
    }
  },

  openBatchRedo: async (project, redo) => {
    const assets = redo.map(r => r.asset);
    set({
      panel: null,
      batch: {
        project,
        assets,
        options: null,
        draft: null,
        initial: null,
        confirming: false,
        error: null,
        redo: true,
      },
    });
    try {
      const [options, jobs] = await Promise.all([
        commands.providerBatchOptions(project, assets),
        commands.providerJobs(project),
      ]);
      const current = get().batch;
      if (current?.project !== project || current.assets !== assets) return;
      const sources: RedoSources = {};
      for (const r of redo) {
        const job = jobs.find(j => j.id === r.job);
        if (job) sources[r.asset] = job;
      }
      if (Object.keys(sources).length === 0) {
        set({ batch: { ...current, error: GONE } });
        return;
      }
      const saved = savedPresets();
      const draft = redoBatchDraft(batchDraft(options, saved.tripo, saved.meshy), sources);
      set({ keys: options.keys, batch: { ...current, options, draft, initial: draft } });
    } catch (error) {
      const current = get().batch;
      if (current?.assets === assets) set({ batch: { ...current, error: errorMessage(error) } });
    }
  },

  setItemPrompt: (asset, prompt) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const items = batch.draft.items.map(i => (i.asset === asset ? { ...i, prompt } : i));
    set({ batch: { ...batch, draft: { ...batch.draft, items }, confirming: false } });
  },
});
