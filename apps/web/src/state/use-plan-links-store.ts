/**
 * The live read of the assets and Lab directions the open canvas links.
 * Nothing of it is in the canvas file: the canvas view asks again when it
 * opens, when its links change, when the project changes, when the window
 * comes back and, while a Lab round runs, every few seconds. A read that
 * answers after a newer one is dropped.
 */
import { create } from 'zustand';
import { commands, errorMessage, type AssetLink, type DirectionLink } from '@/ipc/client';

/** How often the canvas reads again while a linked Lab round runs, in milliseconds. */
export const RUNNING_REREAD_MS = 3_000;

export interface LinkIds {
  assets: readonly string[];
  directions: readonly string[];
}

interface PlanLinksStore {
  project: string | null;
  assets: Readonly<Record<string, AssetLink>>;
  directions: Readonly<Record<string, DirectionLink>>;
  /** Why the last read failed; the nodes keep what the read before said. */
  error: string | null;
  refresh: (project: string, ids: LinkIds) => Promise<void>;
  clear: () => void;
}

let turn = 0;

const byId = <T extends { id: string }>(items: readonly T[]) =>
  Object.fromEntries(items.map(item => [item.id, item]));

export const usePlanLinksStore = create<PlanLinksStore>((set, get) => ({
  project: null,
  assets: {},
  directions: {},
  error: null,

  refresh: async (project, ids) => {
    turn += 1;
    const mine = turn;
    if (get().project !== project) set({ project, assets: {}, directions: {}, error: null });
    if (ids.assets.length === 0 && ids.directions.length === 0) {
      set({ assets: {}, directions: {}, error: null });
      return;
    }
    try {
      const links = await commands.planLinks(project, [...ids.assets], [...ids.directions]);
      if (mine !== turn) return;
      set({ assets: byId(links.assets), directions: byId(links.directions), error: null });
    } catch (error) {
      if (mine === turn) set({ error: errorMessage(error) });
    }
  },

  clear: () => {
    turn += 1;
    set({ project: null, assets: {}, directions: {}, error: null });
  },
}));
