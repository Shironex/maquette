/**
 * The Library's export pack sheet: the source and its settings, the plan
 * the core reads for them, the presets, the pack once it runs (kept up to
 * date by the pack changed event) and the recent packs. The core writes
 * and checks every file; nothing here changes the library.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AssetKind,
  type PackDecision,
  type PackPlan,
  type PackPreset,
  type PackRecord,
  type PackSettings,
  type PackSource,
  type ZipChoice,
} from '@/ipc/client';
import { defaultName } from './packs';
import { useAppStore } from './use-app-store';

interface PackStore {
  open: boolean;
  source: PackSource | null;
  settings: PackSettings | null;
  presets: PackPreset[];
  /** The preset the settings started from. */
  preset: string;
  /** Where the pack goes: null for the workspace's exports folder. */
  out: string | null;
  plan: PackPlan | null;
  error: string | null;
  /** The pack the sheet follows once started, or opened from Recent. */
  record: PackRecord | null;
  recent: PackRecord[];
  busy: boolean;
  /** The pack asked to stop, until it has. */
  stopping: string | null;

  /** Opens the sheet on the settings for `source`. */
  show: (source: PackSource, names?: { collection?: string; project?: string }) => Promise<void>;
  /** Opens the sheet on a pack that ran before. */
  showRecord: (id: string) => Promise<void>;
  close: () => void;
  change: (change: Partial<PackSettings>) => void;
  setBudget: (kind: AssetKind, triangles: number | null) => void;
  pickPreset: (id: string) => void;
  savePreset: (name: string) => Promise<void>;
  pickFolder: () => Promise<void>;
  start: () => Promise<void>;
  decide: (decision: PackDecision) => Promise<void>;
  /** Answers several models in turn; one refused does not stop the rest. */
  decideAll: (decisions: readonly PackDecision[]) => Promise<void>;
  zip: (choice: ZipChoice) => Promise<void>;
  stop: () => Promise<void>;
  resume: (id: string) => Promise<void>;
  reveal: () => Promise<void>;
  /** Back to the settings, for another pack of the same source. */
  again: () => void;
  /** A pack changed: follow it when it is the one on show, and keep Recent fresh. */
  onChanged: (record: PackRecord) => void;
}

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

export const usePackStore = create<PackStore>((set, get) => {
  /** Reads the plan for the settings on show; an older answer is dropped. */
  const replan = async () => {
    const { source, settings } = get();
    if (!source || !settings) return;
    try {
      const plan = await commands.packPlan(source, settings);
      if (get().settings === settings) set({ plan, error: null });
    } catch (error) {
      if (get().settings === settings) set({ plan: null, error: errorMessage(error) });
    }
  };
  /** Runs `job` with the sheet busy; a refusal shows as a notice. */
  const act = async (title: string, job: () => Promise<PackRecord | void>) => {
    set({ busy: true });
    try {
      const record = await job();
      if (record) get().onChanged(record);
    } catch (error) {
      notifyFailed(title, error);
    } finally {
      set({ busy: false });
    }
  };

  return {
    open: false,
    source: null,
    settings: null,
    presets: [],
    preset: 'plain-obj',
    out: null,
    plan: null,
    error: null,
    record: null,
    recent: [],
    busy: false,
    stopping: null,

    show: async (source, names = {}) => {
      set({ open: true, source, record: null, plan: null, error: null, out: null });
      try {
        const [presets, recent] = await Promise.all([
          commands.packPresets(),
          commands.packRecent(),
        ]);
        const first = presets[0];
        const settings = first ? { ...first.settings, name: defaultName(source, names) } : null;
        set({ presets, recent, settings, preset: first?.id ?? 'plain-obj' });
        await replan();
      } catch (error) {
        set({ error: errorMessage(error) });
      }
    },

    showRecord: async id => {
      try {
        const [record, recent] = await Promise.all([commands.packGet(id), commands.packRecent()]);
        set({ open: true, record, recent, source: record.source, settings: record.settings });
      } catch (error) {
        notifyFailed('Could not open the pack', error);
      }
    },

    close: () => set({ open: false, record: null, plan: null, error: null }),

    change: change => {
      const settings = get().settings;
      if (!settings) return;
      set({ settings: { ...settings, ...change } });
      void replan();
    },

    setBudget: (kind, triangles) => {
      const settings = get().settings;
      if (!settings) return;
      const others = settings.budgets.filter(b => b.kind !== kind);
      get().change({ budgets: [...others, { kind, triangles }] });
    },

    pickPreset: id => {
      const preset = get().presets.find(p => p.id === id);
      const name = get().settings?.name ?? '';
      if (!preset) return;
      set({ preset: id, settings: { ...preset.settings, name } });
      void replan();
    },

    savePreset: async name => {
      const settings = get().settings;
      if (!settings) return;
      try {
        const presets = await commands.packSavePreset(name, settings);
        const saved = presets.find(p => p.name === name.trim());
        set({ presets, preset: saved?.id ?? get().preset });
        useAppStore.getState().notify({ status: 'done', title: `Saved the preset ${name.trim()}` });
      } catch (error) {
        notifyFailed('Could not save the preset', error);
      }
    },

    pickFolder: async () => {
      const out = await commands.dialogPickFolder('Where the pack goes', get().out);
      if (out) set({ out });
    },

    start: async () => {
      const { source, settings, out, preset } = get();
      if (!source || !settings) return;
      await act('Could not start the pack', async () => {
        const record = await commands.packStart({ source, settings, out, preset });
        set({ record });
        return record;
      });
    },

    decide: async decision => {
      const id = get().record?.id;
      if (id) await act('Could not change the pack', () => commands.packDecide(id, decision));
    },

    decideAll: async decisions => {
      const id = get().record?.id;
      if (!id) return;
      await act('Could not change the pack', async () => {
        let refused = 0;
        let why: unknown = null;
        for (const decision of decisions) {
          try {
            get().onChanged(await commands.packDecide(id, decision));
          } catch (error) {
            refused += 1;
            why = error;
          }
        }
        if (refused) throw new Error(`${refused} of ${decisions.length}: ${errorMessage(why)}`);
      });
    },

    zip: async choice => {
      const id = get().record?.id;
      if (id) await act('Could not zip the pack', () => commands.packZip(id, choice));
    },

    stop: async () => {
      const id = get().record?.id;
      if (!id) return;
      // Its answer is the record as it stands now, which an event may have
      // passed by the time it arrives, so only the events move the pack on.
      await act('Could not stop the pack', async () => {
        await commands.packStop(id);
        set({ stopping: id });
      });
    },

    resume: async id => {
      await act('Could not go on with the pack', async () => {
        const record = await commands.packResume(id);
        set({ record, open: true, source: record.source, settings: record.settings });
        return record;
      });
    },

    reveal: async () => {
      const id = get().record?.id;
      if (id) await act('Could not show the pack', () => commands.packReveal(id).then(() => {}));
    },

    again: () => {
      const record = get().record;
      set({ record: null });
      if (record) get().change({ name: record.name });
      void commands.packRecent().then(recent => set({ recent }));
    },

    onChanged: record => {
      const shown = get().record;
      const recent = [record, ...get().recent.filter(r => r.id !== record.id)].sort((a, b) =>
        b.createdAt.localeCompare(a.createdAt)
      );
      const stopped = get().stopping === record.id && record.state !== 'running';
      set({
        recent,
        ...(shown?.id === record.id ? { record } : {}),
        ...(stopped ? { stopping: null } : {}),
      });
    },
  };
});
