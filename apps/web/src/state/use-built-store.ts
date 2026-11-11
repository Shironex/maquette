/**
 * The open project's built assets: how each stands against its script and
 * helpers, Add built asset and Rebuild, and putting one in the place of
 * another model. Builds run headless in the app, so a card only ever shows
 * their state; the app (or an agent's build) tells the windows when it
 * changes and the list reloads.
 */
import { create } from 'zustand';
import { commands, errorMessage, type BuiltAsset, type NewBuilt } from '@/ipc/client';
import { outcomeNotice } from './built';
import { replacedNotice, undoneNotice, type ReplaceTarget } from './built-replace';
import { useAppStore } from './use-app-store';

interface BuiltStore {
  /** The project `items` belong to. */
  project: string | null;
  items: readonly BuiltAsset[];
  /** Whether the Add built asset dialog is open. */
  adding: boolean;
  setAdding: (open: boolean) => void;
  load: (project: string) => Promise<void>;
  /** Reloads when `project` is the one shown. */
  onChanged: (project: string) => void;
  /** Builds the asset; `force` runs Blender even when the files match the last build. */
  rebuild: (project: string, asset: string, force: boolean) => Promise<void>;
  /** Adds the asset and starts its build. Rejects with the reason it was refused. */
  add: (project: string, request: NewBuilt) => Promise<void>;
  /** What the replace sheet is open for, when it is. */
  replacing: ReplaceTarget | null;
  setReplacing: (target: ReplaceTarget | null) => void;
  /** Puts the built asset's model in the asset's place. Rejects with why it was refused. */
  replace: (project: string, asset: string, built: string) => Promise<void>;
  /** Brings back the model the asset had before. Rejects with why it was refused. */
  undoReplace: (project: string, asset: string) => Promise<void>;
}

export const useBuiltStore = create<BuiltStore>((set, get) => ({
  project: null,
  items: [],
  adding: false,
  replacing: null,

  setAdding: adding => set({ adding }),
  setReplacing: replacing => set({ replacing }),

  load: async project => {
    if (get().project !== project) set({ project, items: [], adding: false, replacing: null });
    try {
      const items = await commands.builtList(project);
      // A load from a project left meanwhile is not this one's.
      if (get().project === project) set({ items });
    } catch {
      // A project without built assets, or not open yet, shows none.
      if (get().project === project) set({ items: [] });
    }
  },

  onChanged: project => {
    if (get().project === project) void get().load(project);
  },

  rebuild: async (project, asset, force) => {
    const { setProject, notify } = useAppStore.getState();
    const name = get().items.find(b => b.asset === asset)?.name ?? asset;
    try {
      const outcome = await commands.builtRebuild(project, asset, force);
      setProject(outcome.project);
      notify(outcomeNotice(name, outcome));
    } catch (error) {
      notify({ status: 'failed', title: `Could not build ${name}`, detail: errorMessage(error) });
    }
    await get().load(project);
  },

  add: async (project, request) => {
    const added = await commands.builtAdd(project, request);
    useAppStore.getState().setProject(added.project);
    set({ adding: false });
    await get().load(project);
    void get().rebuild(project, added.asset.id, false);
  },

  replace: async (project, asset, built) => {
    const done = await commands.builtReplace(project, asset, built);
    const { setProject, notify } = useAppStore.getState();
    setProject(done.project);
    set({ replacing: null });
    notify(replacedNotice(done.report));
    await get().load(project);
  },

  undoReplace: async (project, asset) => {
    const done = await commands.builtReplaceUndo(project, asset);
    const { setProject, notify } = useAppStore.getState();
    setProject(done.project);
    set({ replacing: null });
    notify(undoneNotice(done.report));
    await get().load(project);
  },
}));
