/**
 * Send to 3D: the open project's provider jobs (kept fresh by
 * `providerUpdate`), the Send to 3D panel and the API key status.
 */
import { create } from 'zustand';
import { commands, errorMessage } from '@/ipc/client';
import { PROVIDER_LABEL, SERVICE_LABEL } from './providers';
import { initialMatches } from './batch';
import { createBatchSlice } from './create-batch-slice';
import { draftFrom, foldIn, notify, upsert } from './providers-store-helpers';
import type { ProvidersStore } from './providers-store.types';

export const useProvidersStore = create<ProvidersStore>((set, get, api) => ({
  ...createBatchSlice(set, get, api),
  project: null,
  jobs: [],
  keys: [],
  panel: null,
  queueOpen: false,
  sending: false,
  busy: [],
  keyBusy: null,
  checklistOpen: false,
  downloads: null,

  load: async project => {
    if (get().project !== project) {
      // The Send to 3D sheets belong to the project they opened in.
      const { panel, batch, downloads } = get();
      set({
        project,
        jobs: [],
        panel: panel && panel.project !== project ? null : panel,
        batch: batch && batch.project !== project ? null : batch,
        downloads: downloads && downloads.project !== project ? null : downloads,
      });
    }
    try {
      const jobs = await commands.providerJobs(project);
      if (get().project === project) set({ jobs });
    } catch (error) {
      notify('failed', 'Could not read the 3D queue', errorMessage(error));
    }
  },

  onEvent: event => {
    if (event.type !== 'job') return;
    const job = event.job;
    if (job.project !== get().project) return;
    const before = get().jobs.find(j => j.id === job.id);
    set(s => ({
      jobs: upsert(s.jobs, job),
      panel: s.panel?.handoff?.id === job.id ? { ...s.panel, handoff: job } : s.panel,
    }));
    if (!before || before.state === job.state) return;
    const name = `${job.assetName}`;
    if (job.state === 'inbox') {
      notify(
        'done',
        `${name} is in the Inbox`,
        `${PROVIDER_LABEL[job.provider]} finished. Check the model and accept it.`
      );
    } else if (job.state === 'failed') {
      notify(
        'failed',
        `${PROVIDER_LABEL[job.provider]} could not make ${name}`,
        job.message ?? undefined
      );
    }
  },

  openPanel: async (project, asset) => {
    set({
      panel: {
        project,
        asset,
        options: null,
        draft: null,
        initial: null,
        handoff: null,
        error: null,
      },
    });
    try {
      const options = await commands.providerSendOptions(project, asset);
      const current = get().panel;
      if (current?.asset !== asset) return;
      const draft = draftFrom(options);
      set({ keys: options.keys, panel: { ...current, options, draft, initial: draft } });
    } catch (error) {
      const current = get().panel;
      if (current?.asset === asset) set({ panel: { ...current, error: errorMessage(error) } });
    }
  },

  closePanel: () => set({ panel: null }),

  setDraft: patch => {
    const panel = get().panel;
    if (!panel?.draft) return;
    let draft = { ...panel.draft, ...patch };
    // A new variant starts from that variant's saved preset.
    if (patch.variant && patch.variant !== panel.draft.variant && panel.options) {
      const fresh = draftFrom({ ...panel.options, variant: patch.variant }, draft.provider);
      draft = { ...draft, tripo: fresh.tripo, meshy: fresh.meshy };
    }
    if (patch.provider && patch.provider !== 'by-hand') draft.service = patch.provider;
    set({ panel: { ...panel, draft } });
  },

  resetDraft: () => {
    const panel = get().panel;
    if (!panel?.draft || !panel.options) return;
    const fresh = draftFrom(
      { ...panel.options, variant: panel.draft.variant },
      panel.draft.provider
    );
    set({
      panel: {
        ...panel,
        draft: { ...panel.draft, tripo: fresh.tripo, meshy: fresh.meshy },
      },
    });
  },

  send: async () => {
    const panel = get().panel;
    const draft = panel?.draft;
    if (!panel || !draft || get().sending) return;
    set({ sending: true });
    try {
      const job = await commands.providerSend({
        project: panel.project,
        asset: panel.asset,
        provider: draft.provider,
        service: draft.service,
        variant: draft.variant,
        images: draft.images,
        tripo: draft.service === 'tripo' ? draft.tripo : null,
        meshy: draft.service === 'meshy' ? draft.meshy : null,
      });
      set(s => ({ jobs: foldIn(s, job) }));
      if (job.provider === 'by-hand') {
        set(s => ({ panel: s.panel ? { ...s.panel, handoff: job } : null }));
      } else {
        set({ panel: null });
        notify(
          'done',
          `${job.assetName} is on its way to ${PROVIDER_LABEL[job.provider]}`,
          'Follow it in the 3D queue. The model lands in the Inbox when it is done.'
        );
      }
    } catch (error) {
      notify('failed', 'Could not send', errorMessage(error));
    } finally {
      set({ sending: false });
    }
  },

  showHandoff: job =>
    set({
      panel: {
        project: job.project,
        asset: job.asset,
        options: null,
        draft: null,
        initial: null,
        handoff: job,
        error: null,
      },
    }),

  setQueueOpen: queueOpen => set({ queueOpen }),

  cancel: async id => {
    set(s => ({ busy: [...s.busy, id] }));
    try {
      const job = await commands.providerCancel(id);
      set(s => ({ jobs: foldIn(s, job) }));
      notify('info', `${job.assetName} cancelled`, job.message ?? undefined);
    } catch (error) {
      notify('failed', 'Could not cancel', errorMessage(error));
    } finally {
      set(s => ({ busy: s.busy.filter(b => b !== id) }));
    }
  },

  handBack: async (id, file) => {
    set(s => ({ busy: [...s.busy, id] }));
    try {
      const job = await commands.providerHandBack(id, file);
      set(s => ({
        jobs: foldIn(s, job),
        panel: s.panel?.handoff?.id === id ? null : s.panel,
      }));
      notify(
        'done',
        `${job.assetName} is in the Inbox`,
        'The checks are running. Accept it there.'
      );
    } catch (error) {
      notify('failed', 'Could not take the model', errorMessage(error));
    } finally {
      set(s => ({ busy: s.busy.filter(b => b !== id) }));
    }
  },

  openHandoff: async id => {
    try {
      await commands.providerOpenHandoff(id);
    } catch (error) {
      notify('failed', 'Could not open the folder', errorMessage(error));
    }
  },

  clearFinished: async () => {
    const project = get().project;
    if (!project) return;
    try {
      const jobs = await commands.providerClearFinished(project);
      if (get().project === project) set({ jobs });
    } catch (error) {
      notify('failed', 'Could not clear the queue', errorMessage(error));
    }
  },

  loadKeys: async () => {
    try {
      set({ keys: await commands.providerKeys() });
    } catch (error) {
      notify('failed', 'Could not read the keychain', errorMessage(error));
    }
  },

  saveKey: async (service, key) => {
    set({ keyBusy: service });
    try {
      const status = await commands.providerKeySave(service, key);
      set(s => ({ keys: [...s.keys.filter(k => k.service !== service), status] }));
      notify(
        'done',
        `${SERVICE_LABEL[service]} key saved`,
        'It is in the keychain, not in any file.'
      );
      return true;
    } catch (error) {
      notify('failed', `Could not save the ${SERVICE_LABEL[service]} key`, errorMessage(error));
      return false;
    } finally {
      set({ keyBusy: null });
    }
  },

  removeKey: async service => {
    set({ keyBusy: service });
    try {
      const status = await commands.providerKeyRemove(service);
      set(s => ({ keys: [...s.keys.filter(k => k.service !== service), status] }));
      notify('info', `${SERVICE_LABEL[service]} key removed`, 'It is gone from the keychain.');
    } catch (error) {
      notify('failed', `Could not remove the ${SERVICE_LABEL[service]} key`, errorMessage(error));
    } finally {
      set({ keyBusy: null });
    }
  },

  testKey: async (service, key) => {
    set({ keyBusy: service });
    try {
      return await commands.providerKeyTest(service, key);
    } catch (error) {
      return { service, ok: false, balance: null, message: errorMessage(error) };
    } finally {
      set({ keyBusy: null });
    }
  },

  setChecklistOpen: checklistOpen => set({ checklistOpen }),

  checkDownloads: async () => {
    const project = get().project;
    if (!project) return;
    set({ downloads: { project, scan: null, error: null, choices: {}, used: [] } });
    try {
      const scan = await commands.providerCheckDownloads(project);
      if (get().downloads?.project !== project) return;
      set({ downloads: { project, scan, error: null, choices: initialMatches(scan), used: [] } });
    } catch (error) {
      if (get().downloads?.project !== project) return;
      set({
        downloads: { project, scan: null, error: errorMessage(error), choices: {}, used: [] },
      });
    }
  },

  chooseMatch: (path, job) => {
    const downloads = get().downloads;
    if (!downloads) return;
    set({ downloads: { ...downloads, choices: { ...downloads.choices, [path]: job } } });
  },

  useDownload: async path => {
    const job = get().downloads?.choices[path];
    if (!job) return;
    await get().handBack(job, path);
    // A failed hand-back keeps the job waiting; only a taken file is done.
    if (get().jobs.find(j => j.id === job)?.state === 'waiting') return;
    const downloads = get().downloads;
    if (downloads) set({ downloads: { ...downloads, used: [...downloads.used, path] } });
  },

  closeDownloads: () => set({ downloads: null }),

  addModel: async id => {
    try {
      const file = await commands.dialogPickModel('Choose the model you made');
      if (file) await get().handBack(id, file);
    } catch (error) {
      notify('failed', 'Could not open the file picker', errorMessage(error));
    }
  },

  openSite: async service => {
    try {
      await commands.providerOpenSite(service);
    } catch (error) {
      notify('failed', `Could not open ${SERVICE_LABEL[service]}`, errorMessage(error));
    }
  },

  revealImage: async (id, image) => {
    try {
      await commands.providerRevealImage(id, image);
    } catch (error) {
      notify('failed', 'Could not show the image', errorMessage(error));
    }
  },
}));
