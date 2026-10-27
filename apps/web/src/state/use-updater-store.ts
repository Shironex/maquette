/**
 * The app update, for the header pill and Settings > About. The shell runs
 * the automatic checks; this store follows its events and starts the steps
 * the user asks for.
 */
import { create } from 'zustand';
import { commands, errorMessage, type UpdaterEvent } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { INITIAL_UPDATER, fromStatus, installing, reduce, type UpdaterState } from './updater';

interface UpdaterStore extends UpdaterState {
  /** Why the last Restart to update was refused, while the update waits. */
  refused: string | null;
  /** The header pill asked Settings to show About. */
  aboutAsked: boolean;
  /** Reads the status and follows the shell's steps; the returned function stops. */
  start: () => () => void;
  apply: (event: UpdaterEvent) => void;
  check: () => Promise<void>;
  download: () => Promise<void>;
  install: () => Promise<void>;
  /** Opens About: Settings scrolls to it, then calls `aboutShown`. */
  reveal: () => void;
  aboutShown: () => void;
}

export const useUpdaterStore = create<UpdaterStore>((set, get) => ({
  ...INITIAL_UPDATER,
  refused: null,
  aboutAsked: false,

  start: () => {
    let stopped = false;
    const listening = events.updaterUpdate.listen(e => get().apply(e.payload));
    commands.updaterStatus().then(
      status => {
        if (!stopped) set(fromStatus(status));
      },
      () => {
        if (!stopped) set({ mode: 'turned-off' });
      }
    );
    return () => {
      stopped = true;
      void listening.then(stop => stop()).catch(() => undefined);
    };
  },

  apply: event => set(s => ({ ...reduce(s, event), refused: null })),

  check: async () => {
    if (get().mode !== 'on') return;
    get().apply({ type: 'checking' });
    try {
      get().apply(await commands.updaterCheck());
    } catch (error) {
      get().apply({ type: 'failed', message: errorMessage(error) });
    }
  },

  download: async () => {
    if (get().mode !== 'on') return;
    try {
      const update = await commands.updaterDownload();
      get().apply({ type: 'ready', update });
    } catch (error) {
      get().apply({ type: 'failed', message: errorMessage(error) });
    }
  },

  install: async () => {
    const ready = get().phase;
    if (ready.kind !== 'ready') return;
    set(s => ({ ...installing(s), refused: null }));
    try {
      // Resolves only if the restart did not happen.
      await commands.updaterInstall();
    } catch (error) {
      // A refusal (work still running) keeps the download: the phase goes
      // back to ready and says why. A failed install already sent its step.
      if (get().phase.kind === 'installing') set({ phase: ready, refused: errorMessage(error) });
    }
  },

  reveal: () => set({ aboutAsked: true }),
  aboutShown: () => set({ aboutAsked: false }),
}));
