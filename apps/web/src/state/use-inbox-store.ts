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
import { openInBlender as openBlender } from './open-in-blender';
import { useAppStore } from './use-app-store';
import { retextureOffer } from './retexture-offer';
import { usePrepareStore } from './use-prepare-store';
import { contentKey, firstToReview, goesTo, itemTitle } from './inbox';
import { isStale, keepRenders, needsSnapshot, withThumbnail } from './thumbs';

interface InboxStore {
  snapshot: InboxSnapshot | null;
  selected: string | null;
  /** Models ticked for a batch redo, in list order. */
  checked: string[];
  loading: boolean;
  /** The item an action is running for. */
  busy: string | null;
  load: () => Promise<void>;
  select: (id: string) => void;
  check: (id: string, on: boolean) => void;
  clearChecked: () => void;
  reprocess: (id: string) => Promise<void>;
  setKind: (id: string, kind: AssetKind) => Promise<void>;
  accept: (id: string, project: string) => Promise<boolean>;
  /** Moves a model aside into the workspace's `inbox/.discarded` folder; false when it failed. */
  discard: (id: string) => Promise<boolean>;
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
  checked: [],
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
        checked: get().checked.filter(id => snapshot.items.some(i => i.id === id)),
      });
      void get().snapshotMissing();
    } catch (error) {
      if (ticket !== loads) return;
      set({ loading: false });
      fail('Could not read the inbox', error);
    }
  },

  select: id => set({ selected: id }),

  check: (id, on) => {
    const order = get().snapshot?.items.map(i => i.id) ?? [];
    const next = new Set(get().checked);
    if (on) next.add(id);
    else next.delete(id);
    set({ checked: order.filter(i => next.has(i)) });
  },

  clearChecked: () => set({ checked: [] }),

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
      // A model from a Send to 3D (or a download attached to an asset) goes to that asset.
      const item = get().snapshot?.items.find(i => i.id === id);
      const goal = item ? goesTo(item) : null;
      const asset = goal && goal.project === project ? goal.asset : null;
      // The Inbox draft replaces the asset's real size and fix-ups; without one it keeps them.
      const prepare = usePrepareStore.getState();
      const prep = prepare.drafts[id];
      const result = await commands.inboxAccept({ item: id, project, asset, name: null, prep });
      if (prep) prepare.setDraft(id, null);
      const kept = prep
        ? (result.note ?? 'Its real size and fix-ups apply on the next run.')
        : null;
      const offer = retextureOffer(item, result.version.version);
      useAppStore.getState().notify({
        status: result.note ? 'info' : 'done',
        title: `${result.asset.name} v${result.version.version} accepted`,
        detail: [`Moved to ${result.version.file}.`, kept, offer?.detail].filter(Boolean).join(' '),
        action: offer?.action,
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

  discard: async id => {
    set({ busy: id });
    try {
      const item = get().snapshot?.items.find(i => i.id === id);
      await commands.inboxDiscard(id);
      useAppStore.getState().notify({
        status: 'done',
        title: `${item ? itemTitle(item) : id} discarded`,
        detail: 'Moved to inbox/.discarded in the workspace, not deleted.',
      });
      await get().load();
      return true;
    } catch (error) {
      fail('Discard failed', error);
      return false;
    } finally {
      set({ busy: null });
    }
  },

  openInBlender: async id => {
    const name = get().snapshot?.items.find(i => i.id === id)?.name ?? id;
    await openBlender(name, () => commands.inboxOpenInBlender(id));
  },

  onThumbnail: event => {
    const snapshot = get().snapshot;
    // A late result for an older file of the same name must not land on the new one.
    if (snapshot && isStale(snapshot, event.file, event)) return;
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
        // The file may have been replaced while its preview was drawn.
        if (snapshot) {
          set({ snapshot: withThumbnail(snapshot, item.file, path, 'ready', item) });
        }
      } catch {
        // The row keeps its kind placeholder; the preview still shows the model.
      }
    }
  },
}));
