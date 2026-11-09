import { create } from 'zustand';
import type { PinCheck } from './recipe-pins';

/**
 * The rough preview's pin checks, measured by the stage and read by each
 * pin's settings ("The foot can stay on its spot"). Empty while no preview
 * plays.
 */
export const useRecipePinChecks = create<{
  checks: PinCheck[];
  set: (checks: PinCheck[]) => void;
}>(set => ({
  checks: [],
  set: checks => set({ checks }),
}));
