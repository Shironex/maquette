/**
 * Send to 3D: the open project's provider jobs (kept fresh by
 * `providerUpdate`), the Send to 3D panel and the API key status.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type BatchOptions,
  type DownloadsScan,
  type KeyStatus,
  type MeshyPreset,
  type PresetVariant,
  type KeyTest,
  type ProviderEvent,
  type ProviderId,
  type ProviderJob,
  type SendOptions,
  type TripoPreset,
} from '@/ipc/client';
import { useAppStore } from './app-store';
import { PROVIDER_LABEL, SERVICE_LABEL, type SendDraft } from './providers';
import {
  batchDraft,
  batchRequest,
  initialMatches,
  presetChanges,
  pruneItems,
  type BatchDraft,
} from './batch';

interface Panel {
  project: string;
  asset: string;
  options: SendOptions | null;
  draft: SendDraft | null;
  /** A By hand send that just wrote its folder: the panel shows its card. */
  handoff: ProviderJob | null;
  error: string | null;
}

interface BatchPanel {
  project: string;
  assets: string[];
  options: BatchOptions | null;
  draft: BatchDraft | null;
  /** An API batch asks once before it spends credits. */
  confirming: boolean;
  error: string | null;
}

interface Downloads {
  project: string;
  scan: DownloadsScan | null;
  error: string | null;
  /** File path to the send it goes to, as the user set it. */
  choices: Record<string, string | null>;
  /** Files already handed back. */
  used: string[];
}

interface ProvidersStore {
  project: string | null;
  jobs: ProviderJob[];
  keys: KeyStatus[];
  panel: Panel | null;
  /** The queue sheet is open. */
  queueOpen: boolean;
  sending: boolean;
  /** Job ids being cancelled or handed back right now. */
  busy: string[];
  /** The service whose key is being saved, removed or tested. */
  keyBusy: ProviderId | null;
  load: (project: string) => Promise<void>;
  onEvent: (event: ProviderEvent) => void;
  openPanel: (project: string, asset: string) => Promise<void>;
  closePanel: () => void;
  setDraft: (patch: Partial<SendDraft>) => void;
  /** Starts over from the saved preset for the draft's variant. */
  resetDraft: () => void;
  send: () => Promise<void>;
  showHandoff: (job: ProviderJob) => void;
  setQueueOpen: (open: boolean) => void;
  cancel: (id: string) => Promise<void>;
  handBack: (id: string, file: string) => Promise<void>;
  openHandoff: (id: string) => Promise<void>;
  clearFinished: () => Promise<void>;
  loadKeys: () => Promise<void>;
  saveKey: (service: ProviderId, key: string) => Promise<boolean>;
  removeKey: (service: ProviderId) => Promise<void>;
  testKey: (service: ProviderId, key: string | null) => Promise<KeyTest | null>;
  batch: BatchPanel | null;
  /** Counts the batches that went out, so the Board can drop its selection. */
  batchesSent: number;
  openBatch: (project: string, assets: string[]) => Promise<void>;
  closeBatch: () => void;
  setBatch: (patch: Partial<Pick<BatchDraft, 'provider' | 'service'>>) => void;
  setItemVariant: (asset: string, variant: PresetVariant) => void;
  /** This batch's copy of one preset; the saved presets stay. */
  setBatchPreset: (
    variant: PresetVariant,
    preset: { tripo?: TripoPreset; meshy?: MeshyPreset }
  ) => void;
  /**
   * One asset's own settings: `preset` is what the form shows, and only
   * its differences from the batch preset are kept. Null clears them.
   */
  setItemPreset: (
    asset: string,
    preset: { tripo?: TripoPreset; meshy?: MeshyPreset } | null
  ) => void;
  /** Starts the batch's presets over from the saved ones. */
  resetBatchPresets: () => void;
  /** Sends the batch; an API batch first asks for a confirm. */
  sendBatch: () => Promise<void>;
  /** The By hand checklist sheet is open. */
  checklistOpen: boolean;
  setChecklistOpen: (open: boolean) => void;
  downloads: Downloads | null;
  checkDownloads: () => Promise<void>;
  chooseMatch: (path: string, job: string | null) => void;
  /** Copies the file into the inbox for its chosen send. The original stays. */
  useDownload: (path: string) => Promise<void>;
  closeDownloads: () => void;
  /** Opens the file picker for a GLB or zip, then hands it back. */
  addModel: (id: string) => Promise<void>;
  openSite: (service: ProviderId) => Promise<void>;
  revealImage: (id: string, image: string) => Promise<void>;
}

const notify = (status: 'done' | 'failed' | 'info', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

function upsert(jobs: readonly ProviderJob[], job: ProviderJob): ProviderJob[] {
  const at = jobs.findIndex(j => j.id === job.id);
  if (at < 0) return [...jobs, job];
  const next = [...jobs];
  next[at] = job;
  return next;
}

/** A draft from the saved presets, with the asset's own refs picked. */
function draftFrom(options: SendOptions, provider: SendDraft['provider'] = 'tripo'): SendDraft {
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
function savedPresets() {
  const providers = useAppStore.getState().workspace?.config.providers;
  if (!providers) throw new Error('Open a workspace first.');
  return { tripo: providers.tripo, meshy: providers.meshy };
}

export const useProvidersStore = create<ProvidersStore>((set, get) => ({
  project: null,
  jobs: [],
  keys: [],
  panel: null,
  queueOpen: false,
  sending: false,
  busy: [],
  keyBusy: null,
  batch: null,
  batchesSent: 0,
  checklistOpen: false,
  downloads: null,

  load: async project => {
    if (get().project !== project) {
      // The Send to 3D panel belongs to the project it opened in.
      const panel = get().panel;
      set({ project, jobs: [], panel: panel && panel.project !== project ? null : panel });
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
    if (before && before.state !== job.state) {
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
    }
  },

  openPanel: async (project, asset) => {
    set({ panel: { project, asset, options: null, draft: null, handoff: null, error: null } });
    try {
      const options = await commands.providerSendOptions(project, asset);
      const current = get().panel;
      if (current?.asset !== asset) return;
      set({
        keys: options.keys,
        panel: { ...current, options, draft: draftFrom(options) },
      });
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
      set(s => ({ jobs: upsert(s.jobs, job) }));
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
        handoff: job,
        error: null,
      },
    }),

  setQueueOpen: queueOpen => set({ queueOpen }),

  cancel: async id => {
    set(s => ({ busy: [...s.busy, id] }));
    try {
      const job = await commands.providerCancel(id);
      set(s => ({ jobs: upsert(s.jobs, job) }));
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
        jobs: upsert(s.jobs, job),
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
      set({ jobs: await commands.providerClearFinished(project) });
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

  openBatch: async (project, assets) => {
    set({
      panel: null,
      batch: { project, assets, options: null, draft: null, confirming: false, error: null },
    });
    try {
      const options = await commands.providerBatchOptions(project, assets);
      const current = get().batch;
      if (current?.project !== project || current.assets !== assets) return;
      const saved = savedPresets();
      set({
        keys: options.keys,
        batch: { ...current, options, draft: batchDraft(options, saved.tripo, saved.meshy) },
      });
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
    set({ batch: { ...batch, draft: pruneItems(draft), confirming: false } });
  },

  setItemPreset: (asset, preset) => {
    const batch = get().batch;
    if (!batch?.draft) return;
    const { draft } = batch;
    const items = draft.items.map(item => {
      if (item.asset !== asset) return item;
      if (!preset) return { ...item, tripo: {}, meshy: {} };
      return {
        ...item,
        ...(preset.tripo && { tripo: presetChanges(draft.tripo[item.variant], preset.tripo) }),
        ...(preset.meshy && { meshy: presetChanges(draft.meshy[item.variant], preset.meshy) }),
      };
    });
    set({ batch: { ...batch, draft: { ...draft, items }, confirming: false } });
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
        jobs: sent.jobs.reduce(upsert, s.jobs),
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
          failed || 'Follow them in the 3D queue. Each model lands in the Inbox.'
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
