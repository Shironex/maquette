import type { StateCreator } from 'zustand';
import { commands, errorMessage } from '@/ipc/client';
import { PROVIDER_LABEL } from './providers';
import { batchDraft, batchRequest, presetChanges, pruneItems, studioFor } from './batch';
import { foldIn, notify, savedPresets } from './providers-store-helpers';
import type { ProvidersStore } from './providers-store.types';

/** The send-many sheet: its panel, per-asset settings and the send itself. */
export type BatchSlice = Pick<
  ProvidersStore,
  | 'batch'
  | 'batchesSent'
  | 'openBatch'
  | 'closeBatch'
  | 'setBatch'
  | 'setItemVariant'
  | 'setBatchPreset'
  | 'setItemPreset'
  | 'resetBatchPresets'
  | 'leaveOutImage'
  | 'sendBatch'
>;

export const createBatchSlice: StateCreator<ProvidersStore, [], [], BatchSlice> = (set, get) => ({
  batch: null,
  batchesSent: 0,

  openBatch: async (project, assets) => {
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
      },
    });
    try {
      const options = await commands.providerBatchOptions(project, assets);
      const current = get().batch;
      if (current?.project !== project || current.assets !== assets) return;
      const saved = savedPresets();
      const draft = batchDraft(options, saved.tripo, saved.meshy);
      set({ keys: options.keys, batch: { ...current, options, draft, initial: draft } });
    } catch (error) {
      const current = get().batch;
      if (current?.assets === assets) set({ batch: { ...current, error: errorMessage(error) } });
    }
  },

  closeBatch: () => set({ batch: null }),

  setBatch: patch => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const draft = { ...batch.draft, ...patch };
    if (patch.provider && patch.provider !== 'by-hand') draft.service = patch.provider;
    set({ batch: { ...batch, draft, confirming: false } });
  },

  setItemVariant: (asset, variant) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const items = batch.draft.items.map(i => (i.asset === asset ? { ...i, variant } : i));
    set({
      batch: { ...batch, draft: pruneItems({ ...batch.draft, items }), confirming: false },
    });
  },

  setBatchPreset: (variant, preset) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const draft = { ...batch.draft };
    if (preset.tripo) draft.tripo = { ...draft.tripo, [variant]: preset.tripo };
    if (preset.meshy) draft.meshy = { ...draft.meshy, [variant]: preset.meshy };
    if (preset.studio) draft.studio = { ...draft.studio, [variant]: preset.studio };
    set({ batch: { ...batch, draft: pruneItems(draft), confirming: false } });
  },

  setItemPreset: (asset, preset) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const { draft } = batch;
    const items = draft.items.map(item => {
      if (item.asset !== asset) return item;
      if (!preset) return { ...item, tripo: {}, meshy: {}, studio: {} };
      return {
        ...item,
        ...(preset.tripo && { tripo: presetChanges(draft.tripo[item.variant], preset.tripo) }),
        ...(preset.meshy && { meshy: presetChanges(draft.meshy[item.variant], preset.meshy) }),
        ...(preset.studio && {
          studio: presetChanges(draft.studio[item.variant], preset.studio),
        }),
      };
    });
    set({ batch: { ...batch, draft: { ...draft, items }, confirming: false } });
  },

  leaveOutImage: (asset, file) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const items = batch.draft.items.map(i =>
      i.asset === asset ? { ...i, images: i.images.filter(f => f !== file) } : i
    );
    set({ batch: { ...batch, draft: { ...batch.draft, items }, confirming: false } });
  },

  resetBatchPresets: () => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const saved = savedPresets();
    set({
      batch: {
        ...batch,
        draft: {
          ...batch.draft,
          tripo: structuredClone(saved.tripo),
          meshy: structuredClone(saved.meshy),
          studio: studioFor(saved.tripo),
        },
        confirming: false,
      },
    });
  },

  sendBatch: async () => {
    const batch = get().batch;
    const draft = batch?.draft;
    if (!batch || !draft || get().sending) return;
    if (draft.provider !== 'by-hand' && !batch.confirming) {
      set({ batch: { ...batch, confirming: true } });
      return;
    }
    set({ sending: true });
    try {
      const sent = await commands.providerSendBatch(batchRequest(batch.project, draft));
      set(s => ({
        jobs: sent.jobs.reduce((jobs, job) => foldIn({ ...s, jobs }, job), s.jobs),
        batch: null,
        batchesSent: sent.jobs.length > 0 ? s.batchesSent + 1 : s.batchesSent,
      }));
      const n = sent.jobs.length;
      const failed = sent.failed.map(f => `${f.asset}: ${f.message}`).join(' ');
      if (draft.provider === 'by-hand') {
        set({ checklistOpen: true });
        notify(
          sent.failed.length ? 'failed' : 'done',
          `${n} ${n === 1 ? 'handoff folder' : 'handoff folders'} written`,
          failed || 'Make each model in the web app, then add it from the checklist.'
        );
      } else {
        notify(
          sent.failed.length ? 'failed' : 'done',
          `${n} ${n === 1 ? 'asset is' : 'assets are'} on the way to ${PROVIDER_LABEL[draft.provider]}`,
          failed ||
            (batch.redo
              ? 'Follow them in the 3D queue. The earlier models stay in the Inbox to compare.'
              : 'Follow them in the 3D queue. Each model lands in the Inbox.')
        );
      }
    } catch (error) {
      const current = get().batch;
      if (current) set({ batch: { ...current, confirming: false } });
      notify('failed', 'Could not send the batch', errorMessage(error));
    } finally {
      set({ sending: false });
    }
  },
});
