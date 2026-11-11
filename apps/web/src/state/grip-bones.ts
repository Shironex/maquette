/**
 * The bones of a grip by group, as the save a grip sheet and the grip
 * panels count them: each side's hand, fingers, forearm and upper arm.
 * Pure helpers.
 */
import type { GripSide } from '@/ipc/client';
import { boneSide } from './recipe-grips';

/** A group of a grip's bones: "Right fingers" and the bones in it. */
export interface GripBoneGroup {
  id: string;
  label: string;
  bones: string[];
}

const PARTS = [
  ['Hand', 'hand'],
  ['fingers', 'fingers'],
  ['LowerArm', 'forearm'],
  ['UpperArm', 'upper arm'],
] as const;

const partOf = (bone: string, side: GripSide) => {
  const rest = bone.slice(side.length);
  if (rest === 'Hand') return 'Hand';
  if (rest === 'LowerArm' || rest === 'UpperArm') return rest;
  return 'fingers';
};

/**
 * `bones` grouped, the holding side `first` (the right unless told), each
 * side's hand, fingers, forearm, then upper arm; a group with no bone is
 * left out. A bone on neither side is left out too.
 */
export function gripBoneGroups(
  bones: readonly string[],
  first: GripSide = 'right'
): GripBoneGroup[] {
  const sides: GripSide[] = first === 'right' ? ['right', 'left'] : ['left', 'right'];
  return sides.flatMap(side =>
    PARTS.flatMap(([part, word]) => {
      const inGroup = bones.filter(b => boneSide(b) === side && partOf(b, side) === part);
      if (!inGroup.length) return [];
      const label = `${side === 'right' ? 'Right' : 'Left'} ${word}`;
      return [{ id: `${side}-${part}`, label, bones: inGroup }];
    })
  );
}
