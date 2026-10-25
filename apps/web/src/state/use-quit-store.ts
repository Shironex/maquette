/**
 * The question asked when the window is closed with Compression presets or
 * material tweaks not applied yet: apply and quit, quit without applying,
 * or stay.
 */
import { create } from 'zustand';
import { commands } from '@/ipc/client';
import { useMaterialsStore } from './use-materials-store';
import { usePipelineStore } from './use-pipeline-store';

interface QuitStore {
  /** The shell held a close back and the dialog is open. */
  asking: boolean;
  applying: boolean;
  /** Why Apply and quit did not close the window. */
  error: string | null;
  ask: () => void;
  cancel: () => void;
  /** Saves every unapplied preset and tweak, then closes; stays open if a save fails. */
  applyAndQuit: () => Promise<void>;
  quitWithoutApplying: () => Promise<void>;
}

/** Projects with unapplied preset rows or material tweaks, each once. */
export const unappliedProjects = (...edits: Record<string, unknown>[]) => [
  ...new Set(edits.flatMap(e => Object.keys(e))),
];

/** The dialog's title for what waits: presets, tweaks or both. */
export function quitTitle(presets: boolean, tweaks: boolean): string {
  if (presets && tweaks) return 'Apply the presets and material tweaks?';
  return tweaks ? 'Apply the material tweaks?' : 'Apply the compression presets?';
}

export const useQuitStore = create<QuitStore>((set, get) => ({
  asking: false,
  applying: false,
  error: null,

  ask: () => set({ asking: true, error: null }),

  cancel: () => {
    if (!get().applying) set({ asking: false, error: null });
  },

  applyAndQuit: async () => {
    if (get().applying) return;
    set({ applying: true, error: null });
    const problem =
      (await usePipelineStore.getState().applyAllEdits()) ??
      (await useMaterialsStore.getState().applyAll());
    set({ applying: false, error: problem });
    if (!problem) await commands.quitNow();
  },

  quitWithoutApplying: async () => {
    if (!get().applying) await commands.quitNow();
  },
}));
