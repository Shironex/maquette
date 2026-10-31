/**
 * A website project's Ship settings (`ship.json` and the site folders in
 * `maquette.site.json`), the dry-run plan and the size report.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type ShipSave_Deserialize as ShipSave,
  type ShipView_Serialize as ShipView,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';

interface ShipStore {
  view: ShipView | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  load: (project: string) => Promise<void>;
  /** Saves a change and takes the new plan; false when it was refused. */
  save: (project: string, change: ShipSave) => Promise<boolean>;
}

export const useShipStore = create<ShipStore>((set, get) => ({
  view: null,
  loading: false,
  saving: false,
  error: null,

  load: async project => {
    if (get().view?.project !== project) set({ view: null });
    set({ loading: true, error: null });
    try {
      const view = await commands.shipGet(project);
      set({ view, loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
    }
  },

  save: async (project, change) => {
    set({ saving: true });
    try {
      const view = await commands.shipSave(project, change);
      // Ship may be showing another project by now.
      if (get().view?.project === project) set({ view });
      return true;
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'Could not save the Ship settings',
        detail: errorMessage(error),
      });
      return false;
    } finally {
      set({ saving: false });
    }
  },
}));
