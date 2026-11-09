import type { StageGizmo } from '@/features/viewer/RecipeStage';
import { turnedKey } from '@/features/viewer/turn-gizmo';
import { HUMAN_BONES } from '@/state/recipe-bones';
import { usePoseKeyEditing } from '@/state/use-pose-key-editing';

/** The body's bones the preview puts a dot on: no fingers, eyes or jaw, which the list picks. */
export const DOT_BONES = HUMAN_BONES.filter(b => !/Eye|jaw|Thumb|Index|Middle|Ring|Little/.test(b));

/** The turn gizmo's props while a pose key change is picked, else null. */
export function usePoseGizmoProps(): StageGizmo | null {
  const e = usePoseKeyEditing();
  if (!e.layer) return null;
  return {
    bone: e.bone,
    space: e.space,
    turning: e.turning,
    dots: DOT_BONES,
    keyed: e.bones,
    onPick: e.pickBone,
    onTurn: t => e.gizmoTurn(t.phase, from => turnedKey(from, t.axis, t.angle)),
  };
}
