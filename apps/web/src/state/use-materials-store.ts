/**
 * Material tweaks per project: what `materials.json` holds and the edits
 * the Inspector's Material panel has not applied yet. Edits outlive the
 * panel, so switching models, tabs or projects keeps them until Apply,
 * Reset or quitting without applying. Apply sends only the edited assets'
 * looks; the backend lays them over the file on disk, so a load that failed
 * or has not finished never wipes another asset's tweaks.
 */
import { create } from 'zustand';
import { commands, errorMessage, type ModelMaterials, type SceneMaterials } from '@/ipc/client';
import { editLook, type MaterialEdits } from './materials';
import { useAppStore } from './use-app-store';

interface MaterialsStore {
  /** The workspace the projects below belong to. */
  root: string | null;
  /** What is saved, by project. Missing until it loads. */
  saved: Readonly<Record<string, SceneMaterials>>;
  /** Unapplied edits, by project and asset. */
  edits: Readonly<Record<string, MaterialEdits>>;
  /** Why the saved tweaks did not load, by project. */
  failed: Readonly<Record<string, string>>;
  saving: boolean;
  /**
   * Follows the open workspace. Project ids repeat across workspaces, so
   * another root drops what is saved and the edits.
   */
  setRoot: (root: string | null) => void;
  /** Reads the saved tweaks; edits an agent's save made redundant drop. */
  load: (project: string) => Promise<void>;
  setLook: (project: string, asset: string, look: ModelMaterials) => void;
  /** Drops the asset's unapplied edits. */
  reset: (project: string, asset: string) => void;
  /** Saves every edit of `project`; true when they were saved. */
  apply: (project: string) => Promise<boolean>;
  /**
   * Saves the edits of every project, one after the other. Resolves to null
   * when all are saved, else to what went wrong (the rest stay unapplied).
   */
  applyAll: () => Promise<string | null>;
}

function without<T>(map: Readonly<Record<string, T>>, key: string): Record<string, T> {
  const { [key]: _gone, ...rest } = map;
  return rest;
}

export const useMaterialsStore = create<MaterialsStore>((set, get) => {
  const setEdits = (project: string, edits: MaterialEdits) =>
    set(s => ({
      edits: Object.keys(edits).length
        ? { ...s.edits, [project]: edits }
        : without(s.edits, project),
    }));

  const save = async (project: string) => {
    const stored = await commands.materialsSave(project, { ...get().edits[project] });
    set(s => ({ saved: { ...s.saved, [project]: stored } }));
    setEdits(project, {});
  };

  return {
    root: null,
    saved: {},
    edits: {},
    failed: {},
    saving: false,

    setRoot: root => {
      if (get().root !== root) set({ root, saved: {}, edits: {}, failed: {} });
    },

    load: async project => {
      const root = get().root;
      try {
        const stored = await commands.materialsGet(project);
        // A load from a workspace closed meanwhile is not this project's.
        if (get().root !== root) return;
        set(s => ({
          saved: { ...s.saved, [project]: stored },
          failed: without(s.failed, project),
        }));
        // An edit an agent's save now matches is no edit any more.
        const edits = Object.entries(get().edits[project] ?? {});
        setEdits(
          project,
          edits.reduce<MaterialEdits>(
            (out, [asset, look]) => editLook(stored, out, asset, look),
            {}
          )
        );
      } catch (error) {
        if (get().root !== root) return;
        set(s => ({ failed: { ...s.failed, [project]: errorMessage(error) } }));
        useAppStore.getState().notify({
          status: 'failed',
          title: 'Material tweaks did not load',
          detail: errorMessage(error),
        });
      }
    },

    setLook: (project, asset, look) =>
      setEdits(
        project,
        editLook(get().saved[project] ?? null, get().edits[project] ?? {}, asset, look)
      ),

    reset: (project, asset) => setEdits(project, without(get().edits[project] ?? {}, asset)),

    apply: async project => {
      if (get().saving) return false;
      set({ saving: true });
      try {
        await save(project);
        useAppStore.getState().notify({
          status: 'done',
          title: 'Material tweaks saved',
          detail: 'Compress applies them to the shipped model on the next run.',
        });
        return true;
      } catch (error) {
        useAppStore.getState().notify({
          status: 'failed',
          title: 'Material tweaks not saved',
          detail: errorMessage(error),
        });
        return false;
      } finally {
        set({ saving: false });
      }
    },

    applyAll: async () => {
      for (const project of Object.keys(get().edits)) {
        try {
          await save(project);
        } catch (error) {
          const name = useAppStore.getState().workspace?.projects.find(p => p.id === project)?.name;
          return `${name ?? project}: ${errorMessage(error)}`;
        }
      }
      return null;
    },
  };
});
