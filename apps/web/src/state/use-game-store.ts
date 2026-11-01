/**
 * A game project's scan, import, settings and Ship plan: the Import models
 * dialog, the Budget panel and the Ship panel read and change them here.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type Asset,
  type GameScan,
  type GameSettingsChange,
  type GameTemplateId,
  type GameTemplatePreview,
  type ShipPlan,
  type ThumbnailReady,
} from '@/ipc/client';
import { isBoardModel, wantsPreview } from './thumbs';
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
  /** Previews queued in Blender that have not finished yet, for `waitingFor`. */
  waiting: number;
  waitingFor: string | null;
  /** Previews that failed since the last notice about them. */
  failedPreviews: number;
  /** The models those were of, so their cards stop saying they render. */
  failedFiles: string[];
  openImport: (project: string) => Promise<void>;
  closeImport: () => void;
  toggle: (file: string, on: boolean) => void;
  pickAll: (on: boolean) => void;
  runImport: () => Promise<boolean>;
  saveSettings: (project: string, change: GameSettingsChange) => Promise<boolean>;
  /** What a template would change; null (with a notice) when it cannot say. */
  previewTemplate: (project: string, id: GameTemplateId) => Promise<GameTemplatePreview | null>;
  applyTemplate: (project: string, id: GameTemplateId) => Promise<boolean>;
  addBudgetFixups: (project: string) => Promise<void>;
  loadPlan: (project: string) => Promise<void>;
  /** Asks for a preview of every model without one; the cards update as they finish. */
  renderPreviews: (project: string) => Promise<void>;
  /** Counts a finished render (from the `thumbnailReady` event) against Render previews. */
  onThumbnail: (event: ThumbnailReady) => void;
}

const plural = (n: number) => `${n} ${n === 1 ? 'preview' : 'previews'}`;

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
  waiting: 0,
  waitingFor: null,
  failedPreviews: 0,
  failedFiles: [],

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

  previewTemplate: async (project, id) => {
    try {
      return await commands.gameTemplatePreview(project, id);
    } catch (error) {
      fail('Could not read that template', error);
      return null;
    }
  },

  applyTemplate: async (project, id) => {
    set({ saving: true });
    try {
      useAppStore.getState().setProject(await commands.gameApplyTemplate(project, id));
      void usePipelineStore.getState().refresh(project);
      return true;
    } catch (error) {
      fail('Could not apply the template', error);
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
    if (get().rendering || get().waiting > 0) return;
    set({ rendering: true, failedPreviews: 0, failedFiles: [] });
    try {
      const {
        asked,
        rendering,
        failed,
        reason,
        project: next,
      } = await commands.thumbnailRenderMissing(project);
      const app = useAppStore.getState();
      app.setProject(next);
      set({ waiting: rendering, waitingFor: project, failedPreviews: 0 });
      if (failed > 0) {
        app.notify({
          status: 'failed',
          title: `${plural(failed)} could not render`,
          detail: `${reason ?? 'Blender could not render them.'} Render previews to retry.`,
        });
      } else {
        app.notify({
          status: 'done',
          title: asked ? `Rendering ${plural(asked)}` : 'Every model has a preview',
          detail: asked
            ? 'Blender renders them one at a time; the cards update as each one is done.'
            : undefined,
        });
      }
    } catch (error) {
      fail('Could not render the previews', error);
    } finally {
      set({ rendering: false });
    }
  },

  onThumbnail: event => {
    const project = useAppStore.getState().workspace?.project;
    // One event per file ends a render; the ones with a project only apply it.
    if (!project || event.project || !isBoardModel(project, event.file)) return;
    const counted = get().waitingFor === project.id && get().waiting > 0;
    const waiting = counted ? get().waiting - 1 : get().waiting;
    const failed = get().failedPreviews + (event.state === 'fallback' ? 1 : 0);
    const done = waiting === 0;
    const failedFiles =
      event.state === 'fallback' ? [...get().failedFiles, event.file] : get().failedFiles;
    set({ waiting, failedPreviews: done ? 0 : failed, failedFiles: done ? [] : failedFiles });
    if (done && failed > 0) {
      useAppStore.getState().notify({
        status: 'failed',
        title: `${plural(failed)} could not render`,
        detail: 'Render previews to retry.',
      });
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

/** True while Render previews is on this card's model, as the Inbox's thumbnails say theirs. */
export function useRendersPreview(asset: Asset, project: string | undefined): boolean {
  return useGameStore(s => {
    if (!s.rendering && !(s.waiting > 0 && s.waitingFor === project)) return false;
    if (!wantsPreview(asset)) return false;
    const file = asset.versions[asset.versions.length - 1]?.file ?? '';
    return !s.failedFiles.some(f => f.replaceAll('\\', '/').endsWith(file));
  });
}
