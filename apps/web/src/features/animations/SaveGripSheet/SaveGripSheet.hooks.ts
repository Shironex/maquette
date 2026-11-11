import type { GripHands } from '@/ipc/client';
import { gripBoneGroups } from '@/state/grip-bones';
import { WEAPONS } from '@/state/recipe-grips';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { gripOfSheet, useGripSaveStore } from '@/state/use-grip-save-store';

/** The target picker's value for a new library grip, never a grip id. */
export const NEW_GRIP = '(new)';

const SIDE_WORDS = { left: 'left', right: 'right' } as const;

/** The open sheet, its choices as the fields show them, and the actions. */
export function useSaveGripSheet() {
  const sheet = useGripSaveStore(s => s.sheet);
  const list = useGripLibraryStore(s => s.list);
  const { choose, save, close } = useGripSaveStore.getState();
  const plan = sheet?.plan ?? null;
  const pose = sheet?.pose ?? {};
  const all = Object.keys(pose);
  const groups = gripBoneGroups(all).map(g => ({
    ...g,
    kept: g.bones.every(b => sheet?.bones.includes(b)),
  }));
  const grip = sheet ? gripOfSheet(sheet) : null;
  const kept = all.filter(b => sheet?.bones.includes(b));
  const sides = {
    left: kept.some(b => b.startsWith('left')),
    right: kept.some(b => b.startsWith('right')),
  };
  const missing =
    sheet && !grip && sheet.pose
      ? (['right', 'left'] as const).find(
          side => (sheet.hands === 'both' || sheet.hands === side) && !sides[side]
        )
      : undefined;
  return {
    sheet,
    plan,
    grip,
    groups,
    /** Why there is no grip to plan: a hand asked for that the pose does not turn, or no name. */
    waiting: !sheet?.pose
      ? null
      : !kept.length
        ? 'Keep at least one bone.'
        : missing
          ? `The pose keys turn no bone on the ${SIDE_WORDS[missing]} hand at frame ${sheet.frame}. Pick the other hand, or key it there.`
          : !grip?.name
            ? 'Give the grip a name.'
            : null,
    weapons: WEAPONS.map(([value, label]) => ({ value, label })),
    targets: [
      { value: NEW_GRIP, label: 'A new grip' },
      ...(list ?? [])
        .filter(g => !g.problem)
        .map(g => ({ value: g.id, label: `New revision of ${g.name}` })),
    ],
    target: sheet?.entry ?? NEW_GRIP,
    setTarget: (value: string) => void choose({ entry: value === NEW_GRIP ? null : value }),
    setName: (name: string) => void choose({ name }),
    setWeapon: (weapon: string) => void choose({ weapon }),
    setHands: (hands: GripHands) => void choose({ hands }),
    keepGroup: (bones: readonly string[], keep: boolean) => {
      const now = sheet?.bones ?? [];
      void choose({
        bones: keep ? [...new Set([...now, ...bones])] : now.filter(b => !bones.includes(b)),
      });
    },
    save: () => void save(),
    close,
  };
}
