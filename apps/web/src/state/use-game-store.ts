/**
 * A game project's scan, import, settings and Ship plan: the Import models
 * dialog, the Budget panel and the Ship panel read and change them here.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type GameScan,
  type GameSettingsChange,
  type ShipPlan,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { usePipelineStore } from './use-pipeline-store';

interface GameStore {
  /** The Import models dialog is open for this project. */
  importFor: string | null;
  scan: GameScan | null;
  scanning: boolean;
  /** Model files picked for the import, relative to the model folder. */
  picked: string[];
  importing: boolean;
  plan: ShipPlan | null;
  saving: boolean;
  /** Render previews is asking the thumbnail service. */
  rendering: boolean;
  openImport: (project: string) => Promise<void>;
  closeImport: () => void;
  toggle: (file: string, on: boolean) => void;
  pickAll: (on: boolean) => void;
  runImport: () => Promise<boolean>;
  saveSettings: (project: string, change: GameSettingsChange) => Promise<boolean>;
  addBudgetFixups: (project: string) => Promise<void>;
  loadPlan: (project: string) => Promise<void>;
  /** Asks for a preview of every model without one; the cards update as they finish. */
  renderPreviews: (project: string) => Promise<void>;
}

const fail = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

/** Every model not imported yet, picked by default. */
const fresh = (scan: GameScan) => scan.models.filter(m => !m.asset && !m.error).map(m => m.file);

export const useGameStore = create<GameStore>((set, get) => ({
  importFor: null,
  scan: null,
  scanning: false,
  picked: [],
  importing: false,
  plan: null,
  saving: false,
  rendering: false,

  openImport: async project => {
    set({ importFor: project, scan: null, scanning: true, picked: [] });
    try {
      const scan = await commands.gameScan(project);
      if (get().importFor === project) set({ scan, picked: fresh(scan) });
    } catch (error) {
      fail('Could not scan the game repo', error);
      set({ importFor: null });
    } finally {
      set({ scanning: false });
    }
  },
  closeImport: () => set({ importFor: null, scan: null, picked: [] }),
  toggle: (file, on) =>
    set(s => ({
      picked: on ? [...new Set([...s.picked, file])] : s.picked.filter(f => f !== file),
    })),
  pickAll: on => set(s => ({ picked: on && s.scan ? fresh(s.scan) : [] })),

  runImport: async () => {
    const { importFor, picked, importing } = get();
    if (!importFor || importing || !picked.length) return false;
    set({ importing: true });
    try {
      const result = await commands.gameImport(importFor, picked);
      const app = useAppStore.getState();
      app.setProject(result.project);
      void usePipelineStore.getState().refresh(importFor);
      set({ importFor: null, scan: null, picked: [] });
      app.notify({
        status: 'done',
        title: `${result.imported.length} ${result.imported.length === 1 ? 'model' : 'models'} imported`,
        detail: result.skipped.length
          ? `${result.skipped.length} skipped: ${result.skipped.map(s => `${s.file} (${s.reason})`).join(', ')}.`
          : 'Each one is an asset with the repo file as v1. Their previews render in the background.',
      });
      return true;
    } catch (error) {
      fail('Could not import the models', error);
      return false;
    } finally {
      set({ importing: false });
    }
  },

  saveSettings: async (project, change) => {
    set({ saving: true });
    try {
      useAppStore.getState().setProject(await commands.gameSaveSettings(project, change));
      void usePipelineStore.getState().refresh(project);
      return true;
    } catch (error) {
      fail('Could not save the game settings', error);
      return false;
    } finally {
      set({ saving: false });
    }
  },

  addBudgetFixups: async project => {
    try {
      const changed = await commands.gameAddBudgetFixups(project);
      useAppStore.getState().notify({
        status: 'done',
        title: changed.length
          ? `Fix-ups added to ${changed.length} ${changed.length === 1 ? 'model' : 'models'}`
          : 'Every model fits its budget',
        detail: changed.length ? 'Build models applies them on the next run.' : undefined,
      });
    } catch (error) {
      fail('Could not add the fix-ups', error);
    }
  },

  renderPreviews: async project => {
    if (get().rendering) return;
    set({ rendering: true });
    try {
      const { asked, project: next } = await commands.thumbnailRenderMissing(project);
      useAppStore.getState().setProject(next);
      useAppStore.getState().notify({
        status: 'done',
        title: asked
          ? `Rendering ${asked} ${asked === 1 ? 'preview' : 'previews'}`
          : 'Every model has a preview',
        detail: asked
          ? 'Blender renders them one at a time; the cards update as each one is done.'
          : undefined,
      });
    } catch (error) {
      fail('Could not render the previews', error);
    } finally {
      set({ rendering: false });
    }
  },

  loadPlan: async project => {
    try {
      set({ plan: await commands.gameShipPlan(project) });
    } catch (error) {
      set({ plan: null });
      fail('Could not read what Ship would change', error);
    }
  },
}));
