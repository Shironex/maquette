/**
 * The By hand Downloads watcher: its switch and folder (per machine), the
 * hand-offs it waits for, the downloads it took a look at and the "Which
 * asset is this?" picker. A new model arrives as a quiet notice.
 */
import { create } from 'zustand';
import { commands, errorMessage, type Arrival, type WatchStatus } from '@/ipc/client';
import { arrivalNotice, askingArrivals } from './downloads';
import { useAppStore } from './use-app-store';

interface DownloadsStore {
  status: WatchStatus | null;
  saving: boolean;
  pickerOpen: boolean;
  /** The send picked for each asking download, by arrival id. */
  choices: Record<string, string>;
  /** Downloads being copied in. */
  busy: string[];
  load: () => Promise<void>;
  setWatch: (enabled: boolean, folder: string | null) => Promise<void>;
  /** Asks for a folder to watch instead of Downloads. */
  pickFolder: () => Promise<void>;
  onArrival: (arrival: Arrival) => void;
  openPicker: () => void;
  closePicker: () => void;
  choose: (arrival: string, job: string) => void;
  assign: (arrival: string) => Promise<void>;
  dismiss: (arrival: string) => Promise<void>;
}

const appStore = useAppStore.getState;

export const useDownloadsStore = create<DownloadsStore>((set, get) => ({
  status: null,
  saving: false,
  pickerOpen: false,
  choices: {},
  busy: [],

  load: async () => {
    try {
      set({ status: await commands.downloadsWatchStatus() });
    } catch {
      set({ status: null });
    }
  },

  setWatch: async (enabled, folder) => {
    set({ saving: true });
    try {
      set({ status: await commands.downloadsWatchSet(enabled, folder) });
    } catch (error) {
      appStore().notify({
        status: 'failed',
        title: 'Could not change the Downloads watch',
        detail: errorMessage(error),
      });
    } finally {
      set({ saving: false });
    }
  },

  pickFolder: async () => {
    const start = get().status?.folder ?? null;
    const picked = await commands
      .dialogPickFolder('Watch this folder for models', start)
      .catch(() => null);
    if (picked) await get().setWatch(get().status?.enabled ?? true, picked);
  },

  onArrival: arrival => {
    set(s => ({
      status: s.status
        ? {
            ...s.status,
            arrivals: [arrival, ...s.status.arrivals.filter(a => a.id !== arrival.id)],
          }
        : s.status,
    }));
    void get().load();
    const notice = arrivalNotice(arrival);
    if (!notice) return;
    appStore().notify({
      status: notice.status,
      title: notice.title,
      detail: notice.detail,
      action: notice.ask ? { label: 'Choose', run: () => get().openPicker() } : undefined,
    });
  },

  openPicker: () => {
    set({ pickerOpen: true });
    void get().load();
  },

  closePicker: () => set({ pickerOpen: false }),

  choose: (arrival, job) => set(s => ({ choices: { ...s.choices, [arrival]: job } })),

  assign: async arrival => {
    const job = get().choices[arrival];
    if (!job) return;
    set(s => ({ busy: [...s.busy, arrival] }));
    try {
      get().onArrival(await commands.downloadsAssign(arrival, job));
      if (askingArrivals(get().status).length === 0) set({ pickerOpen: false });
    } catch (error) {
      appStore().notify({
        status: 'failed',
        title: 'Could not add that model',
        detail: errorMessage(error),
      });
    } finally {
      set(s => ({ busy: s.busy.filter(id => id !== arrival) }));
    }
  },

  dismiss: async arrival => {
    try {
      get().onArrival(await commands.downloadsDismiss(arrival));
      if (askingArrivals(get().status).length === 0) set({ pickerOpen: false });
    } catch (error) {
      appStore().notify({
        status: 'failed',
        title: 'Could not skip it',
        detail: errorMessage(error),
      });
    }
  },
}));
