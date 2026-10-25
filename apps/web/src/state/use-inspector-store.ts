/** The Inspector: the project's measured models and what the view shows of them. */
import { create } from 'zustand';
import { commands, errorMessage, type InspectView } from '@/ipc/client';
import type { MaterialMode } from '@/features/viewer/materials';
import { useAppStore } from './use-app-store';
import { pushSample } from './inspector';
import { clock } from '@/lib/clock';

export type InspectMode = Extract<MaterialMode, 'shaded' | 'wireframe' | 'lightmap'>;
/** Compare versions, or lay out the scene. */
export type InspectorTab = 'compare' | 'layout';

interface InspectorStore {
  tab: InspectorTab;
  setTab: (tab: InspectorTab) => void;
  view: InspectView | null;
  /** The project `view` belongs to. */
  project: string | null;
  loading: boolean;
  error: string | null;
  comparison: string | null;
  left: string | null;
  right: string | null;
  selected: string | null;
  mode: InspectMode;
  /** The stop the camera flies to, and a counter so a second click flies again. */
  stop: { index: number; at: number } | null;
  /** Frame time samples in milliseconds, newest last. */
  samples: number[];
  fps: number;
  load: (project: string) => Promise<void>;
  pick: (patch: Partial<Pick<InspectorStore, 'comparison' | 'left' | 'right'>>) => void;
  select: (model: string | null) => void;
  setMode: (mode: InspectMode) => void;
  flyTo: (index: number) => void;
  /** The camera moved by hand, so no stop is current any more. */
  leaveStop: () => void;
  sample: (ms: number, fps: number) => void;
  /** The model stem Export GLB is running for. */
  exporting: string | null;
  /** Re-exports one model through the pipeline's export step. */
  exportGlb: (model: string, label: string) => Promise<void>;
}

export const useInspectorStore = create<InspectorStore>((set, get) => ({
  tab: 'compare',
  setTab: tab => set({ tab }),
  view: null,
  project: null,
  loading: false,
  error: null,
  comparison: null,
  left: null,
  right: null,
  selected: null,
  mode: 'shaded',
  stop: null,
  samples: [],
  fps: 0,
  exporting: null,

  load: async project => {
    const same = get().project === project;
    // What was picked and where the camera stood belong to the project left.
    const fresh = {
      view: null,
      project,
      selected: null,
      comparison: null,
      left: null,
      right: null,
    };
    set({ loading: true, error: null, ...(same ? {} : { ...fresh, stop: null }) });
    try {
      const view = await commands.inspectProject(project);
      if (get().project !== project) return;
      const selected = get().selected;
      set({
        view,
        loading: false,
        selected: selected && view.models.some(m => m.id === selected) ? selected : null,
      });
    } catch (error) {
      if (get().project === project) set({ loading: false, error: errorMessage(error) });
    }
  },
  pick: patch => set(patch),
  select: selected => set({ selected }),
  setMode: mode => set({ mode }),
  flyTo: index => set({ stop: { index, at: clock.nowMs() } }),
  leaveStop: () => {
    if (get().stop) set({ stop: null });
  },
  sample: (ms, fps) => set(s => ({ samples: pushSample(s.samples, ms), fps })),

  exportGlb: async (model, label) => {
    const project = get().project;
    if (!project || get().exporting) return;
    const notify = useAppStore.getState().notify;
    set({ exporting: model });
    try {
      const done = await commands.inspectExportGlb(project, model);
      notify({
        status: 'done',
        title: `Exported ${label} again`,
        detail: `Run ${done.runNumber} wrote ${done.file}. Compress picks it up on the next run.`,
      });
      // Another project may be open by now; it keeps the Inspector.
      if (get().project === project) await get().load(project);
    } catch (error) {
      notify({ status: 'failed', title: `Could not export ${label}`, detail: errorMessage(error) });
    } finally {
      set({ exporting: null });
    }
  },
}));
