/**
 * The Paths editor's state: the project's layout as the app read it, the
 * paths and energize tracks being edited, what is selected and the tool.
 * Every edit saves into layout.json a moment later, like the Timeline view.
 */
import { create } from 'zustand';
import { commands, errorMessage, type LayoutView } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useLayoutStore } from './use-layout-store';
import { useTimelineStore } from './use-timeline-store';
import {
  EMPTY_PATHS,
  draftOf,
  pathsProblem,
  samePaths,
  type PathSelection,
  type PathTool,
  type PathsDraft,
} from './paths';

/** How long after the last edit the draft saves, in milliseconds. */
export const SAVE_DELAY = 500;

interface PathsStore {
  view: LayoutView | null;
  project: string | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  /** Why the last save failed; cleared by the next one that works. */
  saveError: string | null;
  saved: PathsDraft;
  draft: PathsDraft;
  selection: PathSelection;
  tool: PathTool;
  snap: boolean;
  load: (project: string) => Promise<void>;
  /** The layout changed outside the editor. */
  onChanged: (project: string) => Promise<void>;
  select: (selection: PathSelection) => void;
  setTool: (tool: PathTool) => void;
  setSnap: (snap: boolean) => void;
  /** Changes the draft and saves it a moment later. */
  edit: (change: (draft: PathsDraft) => PathsDraft, select?: PathSelection) => void;
  /** Changes the draft without saving, while a point is dragged. */
  preview: (change: (draft: PathsDraft) => PathsDraft) => void;
  /** Saves the draft now. */
  save: () => Promise<boolean>;
}

let requested: string | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export const usePathsStore = create<PathsStore>((set, get) => {
  const take = (view: LayoutView) => {
    const draft = draftOf(view.layout);
    set({ view, saved: draft, draft, loading: false });
    const s = get().selection;
    const path = s ? draft.paths.find(p => p.id === s.path) : undefined;
    if (s && !path)
      set({ selection: draft.paths[0] ? { path: draft.paths[0].id, point: null } : null });
    else if (s && path && s.point !== null && s.point >= path.points.length) {
      set({ selection: { path: s.path, point: null } });
    } else if (!s && draft.paths[0]) set({ selection: { path: draft.paths[0].id, point: null } });
  };

  const later = () => {
    clearTimeout(timer);
    timer = setTimeout(() => void get().save(), SAVE_DELAY);
  };

  return {
    view: null,
    project: null,
    loading: false,
    saving: false,
    error: null,
    saveError: null,
    saved: EMPTY_PATHS,
    draft: EMPTY_PATHS,
    selection: null,
    tool: 'select',
    snap: true,

    load: async project => {
      requested = project;
      if (get().project !== project) {
        clearTimeout(timer);
        set({
          view: null,
          project,
          selection: null,
          saved: EMPTY_PATHS,
          draft: EMPTY_PATHS,
          saveError: null,
        });
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
      if (get().project !== project || get().saving) return;
      // A pending save goes first; its answer is the newest layout.
      if (!samePaths(get().draft, get().saved)) return;
      await get().load(project);
    },

    select: selection => set({ selection }),
    setTool: tool => set({ tool }),
    setSnap: snap => set({ snap }),

    edit: (change, select) => {
      set({ draft: change(get().draft) });
      if (select !== undefined) set({ selection: select });
      later();
    },

    preview: change => {
      clearTimeout(timer);
      set({ draft: change(get().draft) });
    },

    save: async () => {
      clearTimeout(timer);
      const { project, view, draft, saved } = get();
      if (!project || !view || get().saving || samePaths(draft, saved)) return false;
      const problem = pathsProblem(draft);
      if (problem) {
        set({ saveError: problem });
        return false;
      }
      set({ saving: true });
      try {
        const next = await commands.pathsSave(project, draft.paths, draft.tracks);
        if (get().project !== project) return true;
        // Edits made while it saved stay in the draft and save next.
        const changed = get().draft !== draft;
        const kept = get().draft;
        take(next);
        if (changed) {
          set({ draft: kept });
          later();
        }
        set({ saveError: null });
        // The Timeline view and the Layout editor read the same layout.json.
        void useTimelineStore.getState().onChanged(project);
        void useLayoutStore.getState().onChanged(project);
        return true;
      } catch (error) {
        const detail = errorMessage(error);
        set({ saveError: detail });
        useAppStore
          .getState()
          .notify({ status: 'failed', title: 'The paths were not saved', detail });
        return false;
      } finally {
        set({ saving: false });
      }
    },
  };
});
