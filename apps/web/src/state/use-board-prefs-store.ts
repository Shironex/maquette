/**
 * The Board's layout and the list's sort, stage filter and search, per
 * project, remembered on this machine (guarded storage: a private window
 * just starts from the defaults).
 */
import { create } from 'zustand';
import { readJson, writeJson } from '@/lib/storage';
import { DEFAULT_PREFS, parsePrefs, prefsKey, type BoardPrefs } from './board-list';

interface BoardPrefsStore {
  byProject: Readonly<Record<string, BoardPrefs>>;
  update: (project: string, change: Partial<BoardPrefs>) => void;
}

/** What storage held for a project, read once so a selector gets the same object back. */
const read = new Map<string, BoardPrefs>();

export function storedPrefs(project: string): BoardPrefs {
  let prefs = read.get(project);
  if (!prefs) {
    prefs = parsePrefs(readJson(prefsKey(project)));
    read.set(project, prefs);
  }
  return prefs;
}

/** Forgets what was read, for tests that change storage underneath. */
export function forgetStoredPrefs(): void {
  read.clear();
}

export const useBoardPrefsStore = create<BoardPrefsStore>((set, get) => ({
  byProject: {},
  update: (project, change) => {
    const next = { ...(get().byProject[project] ?? storedPrefs(project)), ...change };
    writeJson(prefsKey(project), next);
    set({ byProject: { ...get().byProject, [project]: next } });
  },
}));

/** The prefs of a project, or the defaults without one. */
export function prefsOf(
  byProject: Readonly<Record<string, BoardPrefs>>,
  project: string | undefined
): BoardPrefs {
  if (!project) return DEFAULT_PREFS;
  return byProject[project] ?? storedPrefs(project);
}
