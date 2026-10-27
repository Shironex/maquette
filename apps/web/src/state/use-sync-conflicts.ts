/**
 * The conflict copies the sync tool kept of the open workspace's data
 * files, looked for when the app shows, about once a minute and when the
 * window comes back to the front. Maquette never merges them; the shell
 * names them until they are dealt with or the warning is dismissed.
 */
import { create } from 'zustand';
import { commands, type SyncConflict } from '@/ipc/client';
import { conflictKey } from './sync';

/** How often the workspace is looked at again. */
export const CONFLICT_POLL_MS = 60_000;

interface SyncConflictsStore {
  conflicts: SyncConflict[];
  /** The key of the set the person dismissed. */
  dismissed: string | null;
  refresh: () => Promise<void>;
  dismiss: () => void;
}

export const useSyncConflicts = create<SyncConflictsStore>((set, get) => ({
  conflicts: [],
  dismissed: null,
  refresh: async () => {
    try {
      set({ conflicts: await commands.workspaceSyncConflicts() });
    } catch {
      // No workspace open, or the folder is busy: look again next time.
    }
  },
  dismiss: () => set({ dismissed: conflictKey(get().conflicts) }),
}));
