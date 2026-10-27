/**
 * The open project's workspace trash: Remove from project, the Recently
 * removed list and Restore. A remove changes the Board, the sends and the
 * Inspector's pick, so this store tells each of them.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AssetsRemoved,
  type ProjectManifest,
  type TrashEntry,
} from '@/ipc/client';
import { removedNotice, restoredNotice } from './trash';
import { useAppStore } from './use-app-store';
import { useInspectorStore } from './use-inspector-store';
import { useProvidersStore } from './use-providers-store';

interface TrashStore {
  /** The project `entries` belong to. */
  project: string | null;
  /** Newest first. */
  entries: readonly TrashEntry[];
  /** A restore going now, by trash id. */
  restoring: string | null;
  load: (project: string) => Promise<void>;
  /**
   * Removes the assets and says how it went, with an Undo. Rejects with
   * the reason when the whole remove is refused (a run going, say).
   */
  remove: (project: string, ids: readonly string[]) => Promise<AssetsRemoved>;
  /** Puts removed assets back, in the order given, and says how it went. */
  restore: (project: string, trash: readonly string[]) => Promise<void>;
}

/** The Board, the sends and the Inspector follow a changed project. */
function follow(project: ProjectManifest, gone: readonly string[]) {
  useAppStore.getState().setProject(project);
  void useProvidersStore.getState().load(project.id);
  const inspector = useInspectorStore.getState();
  if (inspector.selected && gone.includes(inspector.selected)) inspector.select(null);
}

export const useTrashStore = create<TrashStore>((set, get) => ({
  project: null,
  entries: [],
  restoring: null,

  load: async project => {
    if (get().project !== project) set({ project, entries: [], restoring: null });
    try {
      const entries = await commands.assetTrashList(project);
      if (get().project === project) set({ entries });
    } catch {
      // A project that cannot be read has nothing to restore.
      if (get().project === project) set({ entries: [] });
    }
  },

  remove: async (project, ids) => {
    const done = await commands.assetRemove(project, [...ids]);
    const removed = done.outcomes.filter(o => o.state === 'removed');
    follow(
      done.project,
      removed.map(o => o.asset)
    );
    const trash = removed.flatMap(o => (o.trash ? [o.trash] : []));
    useAppStore.getState().notify({
      ...removedNotice(done.outcomes),
      ...(trash.length > 0
        ? { action: { label: 'Undo', run: () => void get().restore(project, trash) } }
        : {}),
    });
    await get().load(project);
    return done;
  },

  restore: async (project, trash) => {
    const back: string[] = [];
    const failed: string[] = [];
    for (const id of trash) {
      const name = get().entries.find(e => e.id === id)?.name ?? id;
      set({ restoring: id });
      try {
        const done = await commands.assetRestore(project, id);
        follow(done.project, []);
        back.push(done.asset.name);
      } catch (error) {
        failed.push(`${name}: ${errorMessage(error)}`);
      }
    }
    set({ restoring: null });
    useAppStore.getState().notify(restoredNotice(back, failed));
    await get().load(project);
  },
}));
