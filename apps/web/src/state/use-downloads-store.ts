/**
 * The By hand Downloads watcher: its switch and folder (per machine), the
 * hand-offs it waits for, the downloads it took a look at and the picker
 * where the ones waiting for a look or for their asset are attached to an
 * asset or left alone. A new model arrives as a quiet notice.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type Arrival,
  type DownloadPreview,
  type ThumbnailReady,
  type WatchStatus,
} from '@/ipc/client';
import { arrivalNotice, askingArrivals, waitingArrivals } from './downloads';
import { useAppStore } from './use-app-store';

interface DownloadsStore {
  status: WatchStatus | null;
  saving: boolean;
  pickerOpen: boolean;
  /** The download the asset chooser is open for. */
  chooserFor: string | null;
  /** Each waiting download's thumbnail, by arrival id. */
  previews: Record<string, DownloadPreview>;
  /** Downloads being attached. */
  busy: string[];
  load: () => Promise<void>;
  setWatch: (enabled: boolean, folder: string | null) => Promise<void>;
  /** Asks for a folder to watch instead of Downloads. */
  pickFolder: () => Promise<void>;
  onArrival: (arrival: Arrival) => void;
  openPicker: () => void;
  closePicker: () => void;
  openChooser: (arrival: string) => void;
  closeChooser: () => void;
  /** Asks for a download's thumbnail once; the render may follow as `thumbnailReady`. */
  loadPreview: (arrival: string) => Promise<void>;
  /** Folds in a render that finished (from the `thumbnailReady` event). */
  onThumbnail: (event: ThumbnailReady) => void;
  /**
   * Attaches a download to an asset of the open project: into the Inbox for
   * it, as a named export goes, where accepting it closes the asset's
   * waiting By hand sends. The chooser moves on to the next download asking
   * for its asset. True when it went in.
   */
  attach: (arrival: string, asset: string) => Promise<boolean>;
  dismiss: (arrival: string) => Promise<void>;
}

const appStore = useAppStore.getState;

/** Previews asked for this session, so a failed one is not asked again in a loop. */
const asked = new Set<string>();

export const useDownloadsStore = create<DownloadsStore>((set, get) => {
  /** Closes the picker once nothing waits. */
  const settle = () => {
    if (waitingArrivals(get().status).length === 0) set({ pickerOpen: false, chooserFor: null });
  };

  const forget = (arrival: string) =>
    set(s => ({
      previews: Object.fromEntries(Object.entries(s.previews).filter(([id]) => id !== arrival)),
    }));

  return {
    status: null,
    saving: false,
    pickerOpen: false,
    chooserFor: null,
    previews: {},
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
        action: notice.ask
          ? { label: notice.action ?? 'Choose', run: () => get().openPicker() }
          : undefined,
      });
    },

    openPicker: () => {
      set({ pickerOpen: true });
      void get().load();
    },

    closePicker: () => set({ pickerOpen: false, chooserFor: null }),

    openChooser: arrival => set({ chooserFor: arrival }),

    closeChooser: () => set({ chooserFor: null }),

    loadPreview: async arrival => {
      if (asked.has(arrival)) return;
      asked.add(arrival);
      try {
        const preview = await commands.downloadsPreview(arrival);
        set(s => ({ previews: { ...s.previews, [arrival]: preview } }));
      } catch {
        // No thumbnail (an FBX, a zip without one GLB): the card keeps its icon.
      }
    },

    onThumbnail: event => {
      const entry = Object.entries(get().previews).find(([, p]) => p.file === event.file);
      if (!entry) return;
      const [arrival, preview] = entry;
      set(s => ({
        previews: {
          ...s.previews,
          [arrival]: { ...preview, thumbnail: event.thumbnail, state: event.state },
        },
      }));
    },

    attach: async (arrival, asset) => {
      const project = appStore().workspace?.project?.id;
      if (!project || get().busy.includes(arrival)) return false;
      set(s => ({ busy: [...s.busy, arrival] }));
      try {
        const done = await commands.downloadsAttach(arrival, project, asset);
        forget(arrival);
        get().onArrival(done);
        if (get().chooserFor === arrival) {
          const next = askingArrivals(get().status).find(a => a.id !== arrival);
          set({ chooserFor: next?.id ?? null });
        }
        settle();
        return true;
      } catch (error) {
        appStore().notify({
          status: 'failed',
          title: 'Could not attach that model',
          detail: errorMessage(error),
        });
        return false;
      } finally {
        set(s => ({ busy: s.busy.filter(id => id !== arrival) }));
      }
    },

    dismiss: async arrival => {
      try {
        get().onArrival(await commands.downloadsDismiss(arrival));
        forget(arrival);
        if (get().chooserFor === arrival) set({ chooserFor: null });
        settle();
      } catch (error) {
        appStore().notify({
          status: 'failed',
          title: 'Could not skip it',
          detail: errorMessage(error),
        });
      }
    },
  };
});
