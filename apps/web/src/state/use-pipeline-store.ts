/**
 * The pipeline view of the current project, kept fresh by runner events.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type CompressPreset,
  type PipelineEvent,
  type PipelineView,
} from '@/ipc/client';
import { applyEvent, fileName, starterToast } from './pipeline';
import { presetRows, presetsToSave, withEdits, type PresetEdits } from './presets';
import { useAppStore } from './use-app-store';

interface PipelineStore {
  view: PipelineView | null;
  loading: boolean;
  error: string | null;
  /** True between pressing Run and the run's first event. */
  starting: boolean;
  load: (project: string) => Promise<void>;
  run: (options?: { from?: string | null; force?: boolean }) => Promise<void>;
  cancel: () => Promise<void>;
  savePresets: (presets: CompressPreset[]) => Promise<boolean>;
  /**
   * Preset rows edited in the panel and not applied yet, by project. They
   * outlive the panel, so a run's events or another tab do not drop them.
   */
  presetEdits: Readonly<Record<string, PresetEdits>>;
  setPresetEdits: (project: string, edits: PresetEdits) => void;
  /**
   * Saves the unapplied preset rows of every project, one after the other.
   * Resolves to null when all are saved, else to what went wrong (the rest
   * stay unapplied).
   */
  applyAllEdits: () => Promise<string | null>;
  /** Updates starter scripts; `replace` also replaces edited ones (kept as .bak). */
  updateStarters: (scripts: string[], replace: boolean) => Promise<boolean>;
  /** Reloads the open project's pipeline when `project` is it, for example after an agent saved presets. */
  refresh: (project: string) => Promise<void>;
  onEvent: (event: PipelineEvent) => void;
}

/** The project the latest load asked for; an older answer is dropped. */
let requested: string | null = null;

export const usePipelineStore = create<PipelineStore>((set, get) => ({
  view: null,
  loading: false,
  error: null,
  starting: false,
  presetEdits: {},

  load: async project => {
    requested = project;
    // Another project's graph never shows under this one while it loads.
    if (get().view?.project !== project) set({ view: null, starting: false });
    set({ loading: true, error: null });
    try {
      const view = await commands.pipelineGet(project);
      if (requested === project) set({ view, loading: false });
    } catch (error) {
      if (requested === project) set({ loading: false, error: errorMessage(error) });
    }
  },

  run: async ({ from = null, force = false } = {}) => {
    const view = get().view;
    if (!view) return;
    set({ starting: true });
    try {
      const run = await commands.pipelineRun(view.project, from, force);
      get().onEvent({ type: 'run', run });
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'The run did not start',
        detail: errorMessage(error),
      });
    } finally {
      set({ starting: false });
    }
  },

  cancel: async () => {
    const view = get().view;
    const id = view?.active ?? view?.queued[0]?.id;
    if (!id) return;
    try {
      await commands.pipelineCancel(id);
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'Could not stop the run',
        detail: errorMessage(error),
      });
    }
  },

  savePresets: async presets => {
    const view = get().view;
    if (!view) return false;
    try {
      const next = await commands.pipelineSavePresets(view.project, presets);
      // Saved either way; only the open project's pipeline shows it.
      if (get().view?.project === view.project)
        set({
          view: { ...next, runs: get().view?.runs ?? next.runs, log: get().view?.log ?? next.log },
        });
      useAppStore.getState().notify({ status: 'done', title: 'Presets saved' });
      return true;
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'Presets not saved',
        detail: errorMessage(error),
      });
      return false;
    }
  },

  setPresetEdits: (project, edits) => {
    const { [project]: _old, ...rest } = get().presetEdits;
    set({ presetEdits: Object.keys(edits).length ? { ...rest, [project]: edits } : rest });
  },

  applyAllEdits: async () => {
    for (const [project, edits] of Object.entries(get().presetEdits)) {
      try {
        const open = get().view;
        const view = open?.project === project ? open : await commands.pipelineGet(project);
        const rows = withEdits(presetRows(view), edits);
        const next = await commands.pipelineSavePresets(project, presetsToSave(rows));
        if (get().view?.project === project) {
          set({
            view: {
              ...next,
              runs: get().view?.runs ?? next.runs,
              log: get().view?.log ?? next.log,
            },
          });
        }
        get().setPresetEdits(project, {});
      } catch (error) {
        const name = useAppStore.getState().workspace?.projects.find(p => p.id === project)?.name;
        return `${name ?? project}: ${errorMessage(error)}`;
      }
    }
    return null;
  },

  updateStarters: async (scripts, replace) => {
    const view = get().view;
    if (!view || scripts.length === 0) return false;
    try {
      const next = await commands.pipelineUpdateStarters(view.project, scripts, replace);
      if (get().view?.project !== view.project) return false;
      set({
        view: { ...next, runs: get().view?.runs ?? next.runs, log: get().view?.log ?? next.log },
      });
      const backups = next.starters.flatMap(s => (s.backup ? [fileName(s.backup)] : []));
      useAppStore.getState().notify({
        status: 'done',
        title: starterToast(scripts.length),
        detail: backups.length ? `Your version is kept as ${backups.join(', ')}.` : undefined,
      });
      return true;
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'The scripts were not updated',
        detail: errorMessage(error),
      });
      return false;
    }
  },

  refresh: async project => {
    if (get().view?.project !== project) return;
    try {
      const fresh = await commands.pipelineGet(project);
      if (get().view?.project === project) set({ view: fresh });
    } catch {
      // The next load or run event brings the view back in line.
    }
  },

  onEvent: event => {
    const view = get().view;
    if (!view) return;
    const next = applyEvent(view, event);
    if (next !== view) set({ view: next });
    if (event.type !== 'run' || !['done', 'failed', 'cancelled'].includes(event.run.state)) return;
    // A finished run changed files on disk: refresh artifacts and sizes.
    const project = view.project;
    void commands.pipelineGet(project).then(
      fresh => {
        if (get().view?.project === project) set({ view: fresh });
      },
      () => undefined
    );
  },
}));
