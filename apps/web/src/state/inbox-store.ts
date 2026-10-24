/** The model inbox: its items, the selection and the actions on them. */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AssetKind,
  type InboxItem,
  type InboxSnapshot,
} from '@/ipc/client';
import { useAppStore } from './app-store';
import { firstToReview } from './inbox';

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
}

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
    set({ loading: true });
    try {
      const snapshot = await commands.inboxList();
      const selected = get().selected;
      const keep = snapshot.items.some(i => i.id === selected);
      set({
        snapshot,
        loading: false,
        selected: keep ? selected : (firstToReview(snapshot.items)?.id ?? null),
      });
    } catch (error) {
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
      const result = await commands.inboxAccept({ item: id, project, asset: null, name: null });
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
}));
