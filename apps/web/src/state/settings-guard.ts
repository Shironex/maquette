/**
 * What stops Settings from being left: the lines naming each field that
 * fails validation, and the view a blocked leave was headed for.
 */
import { create } from 'zustand';
import type { ViewId } from '@/features/shell/views';

interface SettingsGuard {
  /** What is wrong in the Settings draft; empty when it can be saved. */
  problems: string[];
  /** The view a blocked leave was headed for; the dialog is open while set. */
  leaveTo: ViewId | null;
  setProblems: (problems: string[]) => void;
  askLeave: (view: ViewId) => void;
  stay: () => void;
}

export const useSettingsGuard = create<SettingsGuard>(set => ({
  problems: [],
  leaveTo: null,
  setProblems: problems => set({ problems }),
  askLeave: leaveTo => set({ leaveTo }),
  stay: () => set({ leaveTo: null }),
}));
