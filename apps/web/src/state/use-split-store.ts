/**
 * The Split into parts and Assemble one prop sheets: which one is open and
 * for what. The Inspector's Parts tab and the Inbox's "One mesh, no parts"
 * check open the split sheet; the Parts tab opens Assemble.
 */
import { create } from 'zustand';

/** The model a split sheet opens on: a version, or an Inbox model of the asset. */
export interface SplitTarget {
  project: string;
  asset: string;
  version: number | null;
  item: string | null;
  /** Runs the free way, the split fix-up in Blender; absent from the Inbox. */
  inBlender?: () => Promise<void> | void;
}

export interface AssembleTarget {
  project: string;
  /** Assets to start the parts list with. */
  parts: string[];
}

interface SplitStore {
  split: SplitTarget | null;
  assemble: AssembleTarget | null;
  openSplit: (target: SplitTarget) => void;
  closeSplit: () => void;
  openAssemble: (target: AssembleTarget) => void;
  closeAssemble: () => void;
}

export const useSplitStore = create<SplitStore>(set => ({
  split: null,
  assemble: null,
  openSplit: split => set({ split }),
  closeSplit: () => set({ split: null }),
  openAssemble: assemble => set({ assemble }),
  closeAssemble: () => set({ assemble: null }),
}));
