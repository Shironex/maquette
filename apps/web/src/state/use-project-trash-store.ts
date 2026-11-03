/**
 * Remove project and Removed projects: a whole project into the workspace
 * trash, the list of removed projects and Restore. Removing the open project
 * opens the next one (or none), as the core decides, so the workspace comes
 * back with every answer.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RemovedProject } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useProjectsStore } from './use-projects-store';

interface ProjectTrashStore {
  /** Newest first. */
  removed: readonly RemovedProject[];
  /** The project the Remove project question is about, by id. */
  asking: string | null;
  /** The Removed projects dialog is open. */
  listOpen: boolean;
  /** A remove or restore going now. */
  busy: boolean;
  load: () => Promise<void>;
  ask: (id: string) => void;
  cancel: () => void;
  openList: () => void;
  closeList: () => void;
  /** Removes the project; the reason when it was refused, else null. */
  remove: (id: string) => Promise<string | null>;
  /** Puts a removed project back and opens it; false when it was refused. */
  restore: (trash: string) => Promise<boolean>;
}

export const useProjectTrashStore = create<ProjectTrashStore>((set, get) => ({
  removed: [],
  asking: null,
  listOpen: false,
  busy: false,

  load: async () => {
    try {
      set({ removed: await commands.projectRemovedList() });
    } catch {
      // No workspace, or a trash that cannot be read: nothing to restore.
      set({ removed: [] });
    }
  },

  ask: id => {
    useProjectsStore.getState().closeMenu();
    set({ asking: id });
  },
  cancel: () => set({ asking: null }),
  openList: () => {
    useProjectsStore.getState().closeMenu();
    set({ listOpen: true });
    void get().load();
  },
  closeList: () => set({ listOpen: false }),

  remove: async id => {
    if (get().busy) return null;
    set({ busy: true });
    try {
      const done = await commands.projectRemove(id);
      const app = useAppStore.getState();
      app.setWorkspace(done.workspace);
      set({ asking: null });
      const trash = done.removed.trash;
      app.notify({
        status: 'done',
        title: `${done.removed.name} is in the workspace trash`,
        detail: 'Removed projects in the project switcher puts it back.',
        action: { label: 'Undo', run: () => void get().restore(trash) },
      });
      void useProjectsStore.getState().load();
      await get().load();
      return null;
    } catch (error) {
      return errorMessage(error);
    } finally {
      set({ busy: false });
    }
  },

  restore: async trash => {
    if (get().busy) return false;
    set({ busy: true });
    const name = get().removed.find(r => r.trash === trash)?.name ?? 'The project';
    const app = useAppStore.getState();
    try {
      app.setWorkspace(await commands.projectRestore(trash));
      app.notify({ status: 'done', title: `${name} is back`, detail: 'It is open now.' });
      set({ listOpen: false });
      void useProjectsStore.getState().load();
      return true;
    } catch (error) {
      app.notify({
        status: 'failed',
        title: `Could not restore ${name}`,
        detail: errorMessage(error),
      });
      return false;
    } finally {
      set({ busy: false });
      await get().load();
    }
  },
}));
