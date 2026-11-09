/**
 * What the pose keys editor works on beside the draft: the bone picked, the
 * gizmo's space, the pose copied, and the bake frame the player stands on
 * (which the player publishes, so the form keys the frame in view). The
 * keys themselves live in the draft, through the editor store's edit.
 */
import { create } from 'zustand';
import type { PoseKey } from '@/ipc/client';

/** The turn gizmo's rings: about the bone's parent (Local) or the world's axes. */
export type GizmoSpace = 'local' | 'world';

interface PoseKeysStore {
  /** The bone the form and the gizmo turn, by VRM name. */
  bone: string;
  space: GizmoSpace;
  /** The gizmo shows (R toggles it). */
  turning: boolean;
  /** A pose copied with Copy pose, to paste on another frame. */
  copied: PoseKey['pose'] | null;
  /** The bake frame the player shows. */
  frame: number;
  /** Moves the player to a bake frame; set by the player while it shows. */
  seek: ((frame: number) => void) | null;

  pickBone: (bone: string) => void;
  setSpace: (space: GizmoSpace) => void;
  setTurning: (turning: boolean) => void;
  copy: (pose: PoseKey['pose'] | null) => void;
  setFrame: (frame: number) => void;
  setSeek: (seek: ((frame: number) => void) | null) => void;
}

export const usePoseKeysStore = create<PoseKeysStore>(set => ({
  bone: 'chest',
  space: 'local',
  turning: true,
  copied: null,
  frame: 0,
  seek: null,

  pickBone: bone => set({ bone }),
  setSpace: space => set({ space }),
  setTurning: turning => set({ turning }),
  copy: copied => set({ copied }),
  setFrame: frame => set(s => (s.frame === frame ? s : { frame })),
  setSeek: seek => set({ seek }),
}));
