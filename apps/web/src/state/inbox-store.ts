/** The model inbox: its items, the selection and the actions on them. */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AssetKind,
  type InboxItem,
  type InboxSnapshot,
  type ThumbnailReady,
} from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useAppStore } from './app-store';
import { contentKey, firstToReview } from './inbox';
import { keepRenders, needsSnapshot, withThumbnail } from './thumbs';

interface InboxStore {
  snapshot: InboxSnapshot | null;
  selected: string | null;
  loading: boolean;
  /** The item an action is running for. */
  busy: string | null;
  load: () => Promise<void>;
  select: (id: string) => void;
  reprocess: (id: string) => Promise<void>;
  setKind: (id: string, kind: AssetKind) => Promise<void>;
  accept: (id: string, project: string) => Promise<boolean>;
  openInBlender: (id: string) => Promise<void>;
  /** Folds in a render that finished (from the `thumbnailReady` event). */
  onThumbnail: (event: ThumbnailReady) => void;
  /** Snapshots the preview of every model Blender will not render, one at a time. */
  snapshotMissing: () => Promise<void>;
}

/**
 * Files already snapshotted (or tried) this session, by content, so a failure
 * is not retried in a loop but a new file under the same name still is.
 */
const snapshotted = new Set<string>();

/** Counts inbox loads, so only the latest one lands. */
let loads = 0;

function replace(snapshot: InboxSnapshot, item: InboxItem): InboxSnapshot {
  return { ...snapshot, items: snapshot.items.map(i => (i.id === item.id ? item : i)) };
}

const fail = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

export const useInboxStore = create<InboxStore>((set, get) => ({
  snapshot: null,
  selected: null,
  loading: false,
  busy: null,

  load: async () => {
    const ticket = ++loads;
    set({ loading: true });
    try {
      const listed = await commands.inboxList();
      // The watcher fires several loads when a model lands; an older answer
      // must not undo a newer one or a render that finished meanwhile.
      if (ticket !== loads) return;
      const before = get().snapshot;
      const snapshot = before ? keepRenders(before, listed) : listed;
      const selected = get().selected;
      const keep = snapshot.items.some(i => i.id === selected);
      set({
        snapshot,
        loading: false,
        selected: keep ? selected : (firstToReview(snapshot.items)?.id ?? null),
      });
      void get().snapshotMissing();
    } catch (error) {
      if (ticket !== loads) return;
      set({ loading: false });
      fail('Could not read the inbox', error);
    }
  },

  select: id => set({ selected: id }),

  reprocess: async id => {
    set({ busy: id });
    try {
      const item = await commands.inboxReprocess(id);
      const snapshot = get().snapshot;
      if (snapshot) set({ snapshot: replace(snapshot, item) });
      void get().snapshotMissing();
    } catch (error) {
      fail('Reprocess failed', error);
    } finally {
      set({ busy: null });
    }
  },

  setKind: async (id, kind) => {
    try {
      const item = await commands.inboxSetKind(id, kind);
      const snapshot = get().snapshot;
      if (snapshot) set({ snapshot: replace(snapshot, item) });
    } catch (error) {
      fail('Could not change the kind', error);
    }
  },

  accept: async (id, project) => {
    set({ busy: id });
    try {
      // A model from a Send to 3D goes to the asset it was sent for.
      const link = get().snapshot?.items.find(i => i.id === id)?.link;
      const asset = link && link.project === project ? link.asset : null;
      const result = await commands.inboxAccept({ item: id, project, asset, name: null });
      useAppStore.getState().notify({
        status: 'done',
        title: `${result.asset.name} v${result.version.version} accepted`,
        detail: `Moved to ${result.version.file}`,
      });
      useAppStore.getState().setProject(result.project);
      await get().load();
      return true;
    } catch (error) {
      fail('Accept failed', error);
      return false;
    } finally {
      set({ busy: null });
    }
  },

  openInBlender: async id => {
    try {
      await commands.inboxOpenInBlender(id);
    } catch (error) {
      fail('Could not open Blender', error);
    }
  },

  onThumbnail: event => {
    const snapshot = get().snapshot;
    if (snapshot) {
      set({ snapshot: withThumbnail(snapshot, event.file, event.thumbnail, event.state) });
    }
    if (event.state === 'fallback') void get().snapshotMissing();
  },

  snapshotMissing: async () => {
    const waiting = (get().snapshot?.items ?? []).filter(
      i => needsSnapshot(i) && !snapshotted.has(contentKey(i))
    );
    if (waiting.length === 0) return;
    const { snapshotModel } = await import('@/features/viewer/snapshot');
    for (const item of waiting) {
      if (snapshotted.has(contentKey(item))) continue;
      snapshotted.add(contentKey(item));
      const url = fileUrl(item.file);
      if (!url) continue;
      try {
        const png = await snapshotModel(url);
        const path = await commands.thumbnailStore(item.file, Array.from(png));
        const snapshot = get().snapshot;
        if (snapshot) set({ snapshot: withThumbnail(snapshot, item.file, path, 'ready') });
      } catch {
        // The row keeps its kind placeholder; the preview still shows the model.
      }
    }
  },
}));
