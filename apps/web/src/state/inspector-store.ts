/** The Inspector: the project's measured models and what the view shows of them. */
import { create } from 'zustand';
import { commands, errorMessage, type InspectView } from '@/ipc/client';
import type { MaterialMode } from '@/features/viewer/materials';
import { pushSample } from './inspector';

export type InspectMode = Extract<MaterialMode, 'shaded' | 'wireframe' | 'lightmap'>;

interface InspectorStore {
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
}

export const useInspectorStore = create<InspectorStore>((set, get) => ({
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

  load: async project => {
    const same = get().project === project;
    set({ loading: true, error: null, ...(same ? {} : { view: null, project, selected: null }) });
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
  flyTo: index => set({ stop: { index, at: Date.now() } }),
  leaveStop: () => {
    if (get().stop) set({ stop: null });
  },
  sample: (ms, fps) => set(s => ({ samples: pushSample(s.samples, ms), fps })),
}));
