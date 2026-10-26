/**
 * The Timeline view's state: the project's timeline as the app read it, the
 * draft being edited, what is selected and the units the ruler counts in.
 * Every edit saves into layout.json a moment later, as the view promises;
 * the playhead is the Page view's, so the page preview follows it.
 */
import { create } from 'zustand';
import { commands, errorMessage, type TimelineView } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useLayoutStore } from './use-layout-store';
import { useMotionStore } from './use-motion-store';
import {
  EMPTY_TIMELINE,
  draftOf,
  sameDraft,
  type TimelineDraft,
  type TimelineSelection,
  type TimelineUnits,
} from './timeline';

/** How long after the last edit the draft saves, in milliseconds. */
export const SAVE_DELAY = 500;

interface TimelineStore {
  view: TimelineView | null;
  project: string | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  /** Why the last save failed; cleared by the next one that works. */
  saveError: string | null;
  saved: TimelineDraft;
  draft: TimelineDraft;
  selection: TimelineSelection;
  units: TimelineUnits;
  load: (project: string) => Promise<void>;
  /** The layout or the project changed outside the view. */
  onChanged: (project: string) => Promise<void>;
  select: (selection: TimelineSelection) => void;
  setUnits: (units: TimelineUnits) => void;
  /** Changes the draft and saves it a moment later. */
  edit: (change: (draft: TimelineDraft) => TimelineDraft, select?: TimelineSelection) => void;
  /** Saves the draft now. */
  save: () => Promise<boolean>;
}

let requested: string | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export const useTimelineStore = create<TimelineStore>((set, get) => {
  const take = (view: TimelineView) => {
    const draft = draftOf(view);
    set({ view, saved: draft, draft, loading: false });
    const s = get().selection;
    const gone =
      (s?.kind === 'state' && !draft.triggers[s.asset]?.[s.index]) ||
      (s?.kind === 'explode' && !draft.timeline.explode[s.asset]) ||
      (s?.kind === 'path' && !draft.timeline.paths[s.path]);
    if (gone) set({ selection: null });
  };

  return {
    view: null,
    project: null,
    loading: false,
    saving: false,
    error: null,
    saveError: null,
    saved: EMPTY_TIMELINE,
    draft: EMPTY_TIMELINE,
    selection: null,
    units: 'percent',

    load: async project => {
      requested = project;
      if (get().project !== project) {
        clearTimeout(timer);
        set({
          view: null,
          project,
          selection: null,
          saved: EMPTY_TIMELINE,
          draft: EMPTY_TIMELINE,
          saveError: null,
        });
      }
      set({ loading: true, error: null });
      try {
        const view = await commands.timelineGet(project);
        if (requested === project) take(view);
      } catch (error) {
        if (requested === project) set({ loading: false, error: errorMessage(error) });
      }
    },

    onChanged: async project => {
      if (get().project !== project || get().saving) return;
      // A pending save goes first; its answer is the newest layout.
      if (!sameDraft(get().draft, get().saved)) return;
      await get().load(project);
    },

    select: selection => set({ selection }),
    setUnits: units => set({ units }),

    edit: (change, select) => {
      set({ draft: change(get().draft) });
      if (select !== undefined) set({ selection: select });
      clearTimeout(timer);
      timer = setTimeout(() => void get().save(), SAVE_DELAY);
    },

    save: async () => {
      clearTimeout(timer);
      const { project, view, draft, saved } = get();
      if (!project || !view || get().saving || sameDraft(draft, saved)) return false;
      set({ saving: true });
      try {
        const next = await commands.timelineSave(project, draft.triggers, draft.timeline);
        if (get().project !== project) return true;
        // Edits made while it saved stay in the draft and save next.
        const later = get().draft !== draft;
        const kept = get().draft;
        take(next);
        if (later) {
          set({ draft: kept });
          timer = setTimeout(() => void get().save(), SAVE_DELAY);
        }
        set({ saveError: null });
        // The Motion view and the Layout editor read the same layout.json.
        void useMotionStore.getState().onChanged(project);
        void useLayoutStore.getState().onChanged(project);
        return true;
      } catch (error) {
        const detail = errorMessage(error);
        set({ saveError: detail });
        useAppStore
          .getState()
          .notify({ status: 'failed', title: 'The timeline was not saved', detail });
        return false;
      } finally {
        set({ saving: false });
      }
    },
  };
});
