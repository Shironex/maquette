/**
 * Codex usage for the status bar and the Lab: the last read, a fresh read
 * every minute while the Lab is open (`poll`), the reads the shell makes
 * after a job settles (`apply`, from its event) and Limit lifted, retry
 * now (`lift`).
 */
import { create } from 'zustand';
import { commands, errorMessage, type CodexUsage, type UsageLift } from '@/ipc/client';

/** How often the open Lab reads the usage again. */
export const POLL_MS = 60_000;

interface CodexUsageStore {
  usage: CodexUsage | null;
  reading: boolean;
  lifting: boolean;
  /** What the last Limit lifted, retry now found. */
  lifted: UsageLift | null;
  error: string | null;
  /** Takes the last read the shell kept, without reading again. */
  load: () => Promise<void>;
  read: () => Promise<void>;
  apply: (usage: CodexUsage) => void;
  /** Reads now and then every minute; the returned function stops. */
  poll: () => () => void;
  lift: () => Promise<UsageLift | null>;
}

export const useCodexUsageStore = create<CodexUsageStore>((set, get) => ({
  usage: null,
  reading: false,
  lifting: false,
  lifted: null,
  error: null,

  load: async () => {
    try {
      const usage = await commands.codexUsageGet();
      if (usage && !get().usage) set({ usage });
    } catch {
      // The status bar simply shows no Codex item.
    }
  },

  read: async () => {
    if (get().reading) return;
    set({ reading: true });
    try {
      set({ usage: await commands.codexUsageRead(), error: null });
    } catch (error) {
      set({ error: errorMessage(error) });
    } finally {
      set({ reading: false });
    }
  },

  apply: usage => set({ usage }),

  poll: () => {
    void get().read();
    const timer = window.setInterval(() => void get().read(), POLL_MS);
    return () => window.clearInterval(timer);
  },

  lift: async () => {
    if (get().lifting) return null;
    set({ lifting: true, lifted: null });
    try {
      const lifted = await commands.labLiftUsageLimit();
      set({ lifted, usage: lifted.usage, error: null });
      return lifted;
    } catch (error) {
      set({ error: errorMessage(error) });
      return null;
    } finally {
      set({ lifting: false });
    }
  },
}));
