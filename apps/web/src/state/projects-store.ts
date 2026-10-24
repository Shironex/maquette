/**
 * The project switcher: the workspace's projects with what runs in them,
 * the menu, the New project dialog and the switch itself.
 */
import { create } from 'zustand';
import { commands, errorMessage, type ProjectSummary } from '@/ipc/client';
import { useAppStore } from './app-store';
import { newProjectRequest, type ProjectFields, type ProjectMode } from './onboarding';

interface ProjectsStore {
  list: ProjectSummary[];
  menuOpen: boolean;
  /** The New project dialog, opened on a template or on Import folder. */
  dialog: ProjectMode | null;
  /** The project a switch is going to. */
  switching: string | null;
  creating: boolean;
  load: () => Promise<void>;
  openMenu: () => void;
  closeMenu: () => void;
  toggleMenu: () => void;
  openDialog: (mode: ProjectMode) => void;
  closeDialog: () => void;
  /** Opens another project; runs and jobs of the one left keep going. */
  switchTo: (id: string) => Promise<boolean>;
  create: (fields: ProjectFields) => Promise<boolean>;
}

const fail = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

export const useProjectsStore = create<ProjectsStore>((set, get) => ({
  list: [],
  menuOpen: false,
  dialog: null,
  switching: null,
  creating: false,

  load: async () => {
    try {
      set({ list: await commands.projectList() });
    } catch (error) {
      fail('Could not list the projects', error);
    }
  },

  openMenu: () => {
    if (get().dialog) return;
    set({ menuOpen: true });
    void get().load();
  },
  closeMenu: () => set({ menuOpen: false }),
  toggleMenu: () => (get().menuOpen ? get().closeMenu() : get().openMenu()),

  openDialog: mode => set({ dialog: mode, menuOpen: false }),
  closeDialog: () => set({ dialog: null }),

  switchTo: async id => {
    const app = useAppStore.getState();
    set({ menuOpen: false });
    if (app.workspace?.project?.id === id || get().switching) return false;
    set({ switching: id });
    try {
      app.setWorkspace(await commands.projectSwitch(id));
      return true;
    } catch (error) {
      fail('Could not open that project', error);
      return false;
    } finally {
      set({ switching: null });
    }
  },

  create: async fields => {
    if (get().creating) return false;
    set({ creating: true });
    try {
      const snapshot = await commands.projectCreate(newProjectRequest(fields));
      const app = useAppStore.getState();
      app.setWorkspace(snapshot);
      app.setView('board', true);
      set({ dialog: null });
      app.notify({
        status: 'done',
        title: `${snapshot.project?.name ?? 'The project'} is ready`,
        detail:
          'Its pipeline and starter scripts are in place. Drop a concept on the Board to begin.',
      });
      return true;
    } catch (error) {
      fail('Could not create the project', error);
      return false;
    } finally {
      set({ creating: false });
    }
  },
}));
