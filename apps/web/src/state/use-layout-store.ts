/**
 * The Layout editor's state: the saved layout, the draft being edited with
 * its undo history, what is selected and how the gizmo snaps.
 */
import { create } from 'zustand';
import { commands, errorMessage, type LayoutView } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { usePipelineStore } from './use-pipeline-store';
import {
  addStop,
  fromLayout,
  layoutProblem,
  moveStop,
  pinAll,
  record,
  redo,
  removeStop,
  sameLayout,
  tidy,
  toLayout,
  undo,
  updateStop,
  type CameraPose,
  type History,
  type PlainLayout,
  type PlainPlacement,
  type PlainStop,
  type Tool,
} from './layout';
import { clock } from '@/lib/clock';

export type Selection = { kind: 'model'; id: string } | { kind: 'stop'; id: string } | null;

const EMPTY: PlainLayout = { models: {}, stops: [], desk: null };

interface LayoutStore {
  view: LayoutView | null;
  project: string | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  /** What was saved, to tell whether the draft has changes. */
  saved: PlainLayout;
  draft: PlainLayout;
  history: History;
  /** An agent changed the layout while the draft had changes of its own. */
  outside: boolean;
  selection: Selection;
  tool: Tool;
  snap: boolean;
  /** The stop the camera looks through, and a counter so a second click flies again. */
  preview: { id: string; at: number } | null;
  /** Where the editor camera stands, for Add stop and Set from view. */
  pose: CameraPose | null;
  load: (project: string) => Promise<void>;
  /** An agent saved the layout of `project`. */
  onChanged: (project: string) => Promise<void>;
  select: (selection: Selection) => void;
  setTool: (tool: Tool) => void;
  setSnap: (snap: boolean) => void;
  setPose: (pose: CameraPose) => void;
  lookThrough: (id: string | null) => void;
  /** Applies a change as one undo step. */
  edit: (change: (draft: PlainLayout) => PlainLayout) => void;
  /** Starts a gizmo drag: one undo step for the whole drag. */
  beginDrag: () => void;
  /** A gizmo drag moved a model, without a new undo step. */
  dragModel: (id: string, placement: PlainPlacement) => void;
  dragStop: (id: string, patch: Partial<PlainStop>) => void;
  placeModel: (id: string, placement: PlainPlacement) => void;
  addStopHere: () => void;
  setStop: (id: string, patch: Partial<Omit<PlainStop, 'id'>>) => void;
  setStopFromView: (id: string) => void;
  moveStop: (id: string, by: -1 | 1) => void;
  removeStop: (id: string) => void;
  setDesk: (desk: [number, number] | null) => void;
  undo: () => void;
  redo: () => void;
  revert: () => void;
  save: () => Promise<boolean>;
}

let requested: string | null = null;

export const useLayoutStore = create<LayoutStore>((set, get) => {
  const take = (view: LayoutView) => {
    const layout = fromLayout(view.layout);
    set({
      view,
      saved: layout,
      draft: layout,
      history: { past: [], future: [] },
      outside: false,
      loading: false,
    });
    const selection = get().selection;
    if (
      (selection?.kind === 'model' && !view.models.some(m => m.asset === selection.id)) ||
      (selection?.kind === 'stop' && !layout.stops.some(s => s.id === selection.id))
    ) {
      set({ selection: null });
    }
  };

  return {
    view: null,
    project: null,
    loading: false,
    saving: false,
    error: null,
    saved: EMPTY,
    draft: EMPTY,
    history: { past: [], future: [] },
    outside: false,
    selection: null,
    tool: 'translate',
    snap: true,
    preview: null,
    pose: null,

    load: async project => {
      requested = project;
      if (get().project !== project) {
        set({ view: null, project, selection: null, preview: null, saved: EMPTY, draft: EMPTY });
      }
      set({ loading: true, error: null });
      try {
        const view = await commands.layoutGet(project);
        if (requested === project) take(view);
      } catch (error) {
        if (requested === project) set({ loading: false, error: errorMessage(error) });
      }
    },

    onChanged: async project => {
      if (get().project !== project) return;
      if (!sameLayout(get().draft, get().saved)) {
        set({ outside: true });
        return;
      }
      await get().load(project);
    },

    select: selection => set({ selection }),
    setTool: tool => set({ tool }),
    setSnap: snap => set({ snap }),
    setPose: pose => set({ pose }),
    lookThrough: id => set({ preview: id ? { id, at: clock.nowMs() } : null }),

    edit: change => {
      const before = get().draft;
      const next = change(before);
      if (sameLayout(before, next)) return;
      set({ draft: next, history: record(get().history, before) });
    },

    beginDrag: () => set({ history: record(get().history, get().draft) }),

    dragModel: (id, placement) => {
      const draft = get().draft;
      set({ draft: { ...draft, models: { ...draft.models, [id]: tidy(placement) } } });
    },

    dragStop: (id, patch) => set({ draft: updateStop(get().draft, id, patch) }),

    placeModel: (id, placement) =>
      get().edit(d => ({ ...d, models: { ...d.models, [id]: tidy(placement) } })),

    addStopHere: () => {
      const pose = get().pose;
      if (!pose) return;
      get().edit(d => addStop(d, pose));
      const added = get().draft.stops[get().draft.stops.length - 1];
      if (added) set({ selection: { kind: 'stop', id: added.id } });
    },

    setStop: (id, patch) => get().edit(d => updateStop(d, id, patch)),

    setStopFromView: id => {
      const pose = get().pose;
      if (!pose) return;
      get().edit(d =>
        updateStop(d, id, { position: pose.position, target: pose.target, fov: pose.fov })
      );
    },

    moveStop: (id, by) => get().edit(d => moveStop(d, id, by)),

    removeStop: id => {
      get().edit(d => removeStop(d, id));
      const { selection, preview } = get();
      if (selection?.kind === 'stop' && selection.id === id) set({ selection: null });
      if (preview?.id === id) set({ preview: null });
    },

    setDesk: desk => get().edit(d => ({ ...d, desk })),

    undo: () => {
      const done = undo(get().history, get().draft);
      if (done) set({ history: done[0], draft: done[1] });
    },

    redo: () => {
      const done = redo(get().history, get().draft);
      if (done) set({ history: done[0], draft: done[1] });
    },

    revert: () =>
      set({ draft: get().saved, history: record(get().history, get().draft), outside: false }),

    save: async () => {
      const { project, view, draft } = get();
      if (!project || !view || get().saving) return false;
      const notify = useAppStore.getState().notify;
      const problem = layoutProblem(draft);
      if (problem) {
        notify({ status: 'failed', title: 'The layout was not saved', detail: problem });
        return false;
      }
      set({ saving: true });
      try {
        const next = await commands.layoutSave(project, toLayout(pinAll(draft, view.models)));
        if (get().project === project) {
          const history = get().history;
          take(next);
          // Saving is not an edit: what could be undone still can.
          set({ history });
        }
        // The Pipeline view and the top bar show the first view too.
        void usePipelineStore.getState().refresh(project);
        return true;
      } catch (error) {
        notify({
          status: 'failed',
          title: 'The layout was not saved',
          detail: errorMessage(error),
        });
        return false;
      } finally {
        set({ saving: false });
      }
    },
  };
});

/** True when the draft has changes that are not saved. */
export function useLayoutDirty(): boolean {
  return useLayoutStore(s => !sameLayout(s.draft, s.saved));
}
