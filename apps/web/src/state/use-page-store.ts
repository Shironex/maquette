/**
 * The Page view: the project's page settings and its site's dev server,
 * the device and layout the view shows, the stop the scrubber asks for and
 * what each page frame reports back through the bridge.
 *
 * Opening the view starts the dev server when nothing else uses it, and
 * closing it lets go: the app stops the server only when the view started
 * it, so one Preview on site or an agent started stays. The server's state
 * arrives as `pageServerChanged` events, kept here even while the view is
 * closed, for the status bar.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type InspectView,
  type PageSettings,
  type PageView,
  type ServerStatus,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { type BridgeStop, type DeviceId, type FromPage, shouldAutoStart } from './page';

export type CompareMode = 'one' | 'ship-run' | 'poster';
export type PageTab = 'preview' | 'devices';

/** What one frame's page said through the bridge. */
export interface Reading {
  /** The page's own stops, once it said ready. */
  stops: BridgeStop[] | null;
  at: number | null;
  scroll: { top: number; max: number } | null;
  stats: { frameMs: number; p95Ms: number; drawCalls: number | null } | null;
}

export const NO_READING: Reading = { stops: null, at: null, scroll: null, stats: null };

/** The frames whose page leads the scrubber, in order. */
const LEADS = ['main', 'ship', 'desktop', 'laptop', 'phone'];

/** The reading of the frame that leads: the one page the scrubber follows. */
export function leadReading(readings: Record<string, Reading>): Reading {
  for (const id of LEADS) {
    const r = readings[id];
    if (r?.stops) return r;
  }
  return NO_READING;
}

/** A release waits this long, so a remount (a quick switch back) keeps the server. */
const RELEASE_DELAY = 400;

interface PageStore {
  project: string | null;
  view: PageView | null;
  inspect: InspectView | null;
  loading: boolean;
  error: string | null;
  /** The app's dev server, whichever project it serves. */
  server: ServerStatus | null;
  device: DeviceId;
  mode: CompareMode;
  tab: PageTab;
  showStops: boolean;
  poster: boolean;
  /** The stop the scrubber asks the frames for, fractional. */
  target: number;
  /** Counts up on each seek, so a frame sends a goto for the same stop again. */
  seekId: number;
  smooth: boolean;
  playing: boolean;
  /** Counts up on Reload, so every frame loads again. */
  reloadKey: number;
  /** Frame id to what its page says. */
  readings: Record<string, Reading>;

  open: (project: string) => Promise<void>;
  close: (project: string) => void;
  refresh: () => Promise<void>;
  onServer: (status: ServerStatus) => void;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  save: (patch: Partial<PageSettings>) => Promise<boolean>;
  pickConcept: () => Promise<void>;
  openInBrowser: () => Promise<void>;
  setDevice: (device: DeviceId) => void;
  setMode: (mode: CompareMode) => void;
  setTab: (tab: PageTab) => void;
  setShowStops: (on: boolean) => void;
  setPoster: (on: boolean) => void;
  reload: () => void;
  seek: (stop: number, smooth?: boolean) => void;
  setPlaying: (playing: boolean) => void;
  onMessage: (frame: string, message: FromPage) => void;
  forgetFrame: (frame: string) => void;
}

const releases = new Map<string, ReturnType<typeof setTimeout>>();

function withServer(view: PageView, status: ServerStatus): PageView {
  const mine = status.project === view.project;
  if (mine) return { ...view, server: status, busyWith: null };
  const idle: ServerStatus = { ...status, state: 'idle', url: null, log: [], message: null };
  return {
    ...view,
    server: idle,
    busyWith: status.state === 'idle' ? null : status.project,
  };
}

export const usePageStore = create<PageStore>((set, get) => {
  const fail = (title: string, error: unknown) =>
    useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

  /** Runs `call` for the open project and keeps its answer when it still is. */
  const apply = async (project: string, call: () => Promise<PageView>) => {
    const view = await call();
    if (get().project === project)
      set({ view, server: view.server.project ? view.server : get().server });
  };

  return {
    project: null,
    view: null,
    inspect: null,
    loading: false,
    error: null,
    server: null,
    device: 'desktop',
    mode: 'one',
    tab: 'preview',
    showStops: true,
    poster: false,
    target: 0,
    seekId: 0,
    smooth: false,
    playing: false,
    reloadKey: 0,
    readings: {},

    open: async project => {
      const pending = releases.get(project);
      if (pending) {
        clearTimeout(pending);
        releases.delete(project);
      }
      if (get().project !== project) {
        set({
          project,
          view: null,
          inspect: null,
          error: null,
          readings: {},
          target: 0,
          playing: false,
        });
      }
      set({ loading: true });
      try {
        const view = await commands.pageGet(project);
        if (get().project !== project) return;
        set({ view, error: null });
        void commands
          .inspectProject(project)
          .then(inspect => {
            if (get().project === project) set({ inspect });
          })
          .catch(() => undefined);
        if (shouldAutoStart(view)) await get().start();
      } catch (error) {
        if (get().project === project) set({ error: errorMessage(error) });
      } finally {
        if (get().project === project) set({ loading: false });
      }
    },

    close: project => {
      set({ playing: false });
      const pending = releases.get(project);
      if (pending) clearTimeout(pending);
      releases.set(
        project,
        setTimeout(() => {
          releases.delete(project);
          void commands.pageServerRelease(project).catch(() => undefined);
        }, RELEASE_DELAY)
      );
    },

    refresh: async () => {
      const project = get().project;
      if (!project) return;
      try {
        await apply(project, () => commands.pageGet(project));
      } catch {
        // The next event or open brings it back.
      }
    },

    onServer: status => {
      set({ server: status });
      const view = get().view;
      if (view) set({ view: withServer(view, status) });
    },

    start: async () => {
      const project = get().project;
      if (!project) return;
      try {
        await apply(project, () => commands.pageServerStart(project));
      } catch (error) {
        fail("Could not start the site's dev server", error);
        await get().refresh();
      }
    },

    stop: async () => {
      const project = get().project;
      if (!project) return;
      try {
        await apply(project, () => commands.pageServerStop(project));
      } catch (error) {
        fail("Could not stop the site's dev server", error);
      }
    },

    save: async patch => {
      const { project, view } = get();
      if (!project || !view) return false;
      try {
        await apply(project, () => commands.pageSave(project, { ...view.settings, ...patch }));
        return true;
      } catch (error) {
        fail('Could not save the page settings', error);
        return false;
      }
    },

    pickConcept: async () => {
      try {
        const file = await commands.dialogPickFile('Pick an HTML page concept');
        if (file) await get().save({ concept: file, source: 'concept' });
      } catch (error) {
        fail('Could not pick the page concept', error);
      }
    },

    openInBrowser: async () => {
      const { project, view } = get();
      if (!project || !view) return;
      try {
        await commands.pageOpenInBrowser(project, view.settings.path);
      } catch (error) {
        fail('Could not open the page in the browser', error);
      }
    },

    setDevice: device => set({ device }),
    setMode: mode => set({ mode }),
    setTab: tab => set({ tab }),
    setShowStops: showStops => set({ showStops }),
    setPoster: poster => set({ poster }),
    reload: () => set(s => ({ reloadKey: s.reloadKey + 1, readings: {} })),

    seek: (stop, smooth = false) =>
      set(s => ({ target: Math.max(0, stop), seekId: s.seekId + 1, smooth })),

    setPlaying: playing => set({ playing }),

    onMessage: (frame, message) =>
      set(s => {
        const before = s.readings[frame] ?? NO_READING;
        const next: Reading =
          message.type === 'ready'
            ? { ...before, stops: message.stops }
            : message.type === 'at'
              ? { ...before, at: message.stop, scroll: message.scroll }
              : {
                  ...before,
                  stats: {
                    frameMs: message.frameMs,
                    p95Ms: message.p95Ms,
                    drawCalls: message.drawCalls,
                  },
                };
        return { readings: { ...s.readings, [frame]: next } };
      }),

    forgetFrame: frame =>
      set(s => {
        if (!(frame in s.readings)) return s;
        const readings = { ...s.readings };
        delete readings[frame];
        return { readings };
      }),
  };
});
