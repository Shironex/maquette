import { boneWords } from '@/state/recipe-bones';
import { usePoseKeyEditing } from '@/state/use-pose-key-editing';
import type { GizmoSpace } from '@/state/use-pose-keys-store';

const AXES = ['X', 'Y', 'Z'] as const;

/** The gizmo's bar under the preview while a pose key change is picked: its space, the turn, and the key here. */
export function usePoseKeysBar() {
  const e = usePoseKeyEditing();
  const turn = e.turn.map((v, i) => `${AXES[i]} ${v.toFixed(1)}°`).join('  ');
  return {
    shown: Boolean(e.layer),
    space: e.space,
    setSpace: (space: GizmoSpace) => e.setSpace(space),
    turning: e.turning,
    toggleTurning: e.toggleTurning,
    readout: `${boneWords(e.bone)}  ${turn}`,
    here: e.here ? `Frame ${e.frame} has a key` : `No key on frame ${e.frame}`,
  };
}
