/**
 * The refs sheet: one asset's refs and their views, opened from its Board
 * card or a ref warning. It also holds the two Lab jobs the sheet starts
 * (Make the other views, Clean background) and reloads as they finish.
 * Every tag change saves at once; the core checks it.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type CleanGround,
  type ConceptEvent,
  type ProjectManifest,
  type RefView,
  type RefsView,
} from '@/ipc/client';
import { assign } from './refs';
import { useAppStore } from './use-app-store';

/** Which of the refs sheets is showing. */
export type RefsMode = 'refs' | 'views' | 'clean';

export interface RefsSheet {
  project: string;
  asset: string;
  mode: RefsMode;
  /** Clean background: the ref it cleans. */
  file: string | null;
}

interface RefsStore {
  sheet: RefsSheet | null;
  view: RefsView | null;
  error: string | null;
  /** True while a change or a job start is on its way. */
  busy: boolean;
  open: (project: string, asset: string, mode?: RefsMode, file?: string | null) => Promise<void>;
  close: () => void;
  /** Switches to another of the sheets for the same asset. */
  show: (mode: RefsMode, file?: string | null) => void;
  reload: () => Promise<void>;
  /** Gives `file` a view; a taken slot swaps with it. */
  tag: (file: string, view: RefView) => Promise<void>;
  /** Adds images from `refs/` as other refs. */
  add: (files: string[]) => Promise<void>;
  /** Drops `file` from the asset's refs; the image stays in `refs/`. */
  remove: (file: string) => Promise<void>;
  makeViews: (views: RefView[], count: number, useStyle: boolean) => Promise<void>;
  clean: (file: string, ground: CleanGround, pad: boolean, extend: boolean) => Promise<void>;
  /** "Use as Left" or "Use the clean copy". */
  use: (file: string, view?: RefView | null) => Promise<void>;
  onConcept: (event: ConceptEvent) => void;
  onProject: (project: ProjectManifest) => void;
}

const notify = (title: string, detail: string) =>
  useAppStore.getState().notify({ status: 'failed', title, detail });

export const useRefsStore = create<RefsStore>((set, get) => {
  /** Runs a change that answers with the refs as they now stand. */
  async function change(title: string, run: (sheet: RefsSheet) => Promise<RefsView>) {
    const sheet = get().sheet;
    if (!sheet || get().busy) return;
    set({ busy: true });
    try {
      const view = await run(sheet);
      useAppStore.getState().setProject(view.project);
      if (get().sheet?.asset === sheet.asset) set({ view });
    } catch (error) {
      notify(title, errorMessage(error));
    } finally {
      set({ busy: false });
    }
  }

  const refsOf = () => get().view?.refs ?? [];

  return {
    sheet: null,
    view: null,
    error: null,
    busy: false,

    open: async (project, asset, mode = 'refs', file = null) => {
      const same = get().view?.asset.id === asset && get().view?.project.id === project;
      set({ sheet: { project, asset, mode, file }, error: null, view: same ? get().view : null });
      await get().reload();
    },

    close: () => set({ sheet: null, view: null, error: null }),

    show: (mode, file = null) => {
      const sheet = get().sheet;
      if (sheet) set({ sheet: { ...sheet, mode, file } });
    },

    reload: async () => {
      const sheet = get().sheet;
      if (!sheet) return;
      try {
        const view = await commands.refsGet(sheet.project, sheet.asset);
        if (get().sheet?.asset === sheet.asset) set({ view, error: null });
      } catch (error) {
        if (get().sheet?.asset === sheet.asset) set({ error: errorMessage(error) });
      }
    },

    tag: (file, view) =>
      change('Could not change the view', s =>
        commands.refsSetViews(s.project, s.asset, assign(refsOf(), file, view))
      ),

    add: files =>
      change('Could not add the refs', s =>
        commands.refsSetViews(
          s.project,
          s.asset,
          files.reduce((refs, file) => assign(refs, file, 'other'), refsOf())
        )
      ),

    remove: file =>
      change('Could not leave the ref out', s =>
        commands.refsSetViews(
          s.project,
          s.asset,
          refsOf().filter(r => r.file !== file)
        )
      ),

    makeViews: (views, count, useStyle) =>
      change('Could not start the views', s =>
        commands.refsMakeViews({ project: s.project, asset: s.asset, views, count, useStyle })
      ),

    clean: (file, ground, pad, extend) =>
      change('Could not start Clean background', s =>
        commands.refsClean({ project: s.project, asset: s.asset, file, ground, pad, extend })
      ),

    use: (file, view = null) =>
      change('Could not use the image', s => commands.refsUse(s.project, s.asset, file, view)),

    onConcept: event => {
      const view = get().view;
      if (!view || !get().sheet) return;
      const ours = (id: string) => view.directions.some(d => d.id === id);
      const done = event.type === 'job' && ours(event.job.direction) && event.job.state === 'done';
      const direction =
        event.type === 'direction' && event.direction.refsJob?.asset === view.asset.id;
      if (done || direction) void get().reload();
      else if (event.type === 'job' && ours(event.job.direction))
        set({
          view: {
            ...view,
            jobs: [...view.jobs.filter(j => j.id !== event.job.id), event.job],
          },
        });
    },

    onProject: project => {
      if (get().sheet?.project === project.id) void get().reload();
    },
  };
});
