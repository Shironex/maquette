/**
 * Send to 3D: the open project's provider jobs (kept fresh by
 * `providerUpdate`), the Send to 3D panel and the API key status.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type KeyStatus,
  type KeyTest,
  type ProviderEvent,
  type ProviderId,
  type ProviderJob,
  type SendOptions,
} from '@/ipc/client';
import { useAppStore } from './app-store';
import { PROVIDER_LABEL, SERVICE_LABEL, type SendDraft } from './providers';

interface Panel {
  project: string;
  asset: string;
  options: SendOptions | null;
  draft: SendDraft | null;
  /** A By hand send that just wrote its folder: the panel shows its card. */
  handoff: ProviderJob | null;
  error: string | null;
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

export const useProvidersStore = create<ProvidersStore>((set, get) => ({
  project: null,
  jobs: [],
  keys: [],
  panel: null,
  queueOpen: false,
  sending: false,
  busy: [],
  keyBusy: null,

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
}));
