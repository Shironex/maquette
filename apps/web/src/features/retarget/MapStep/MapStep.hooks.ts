import { useState } from 'react';
import { RIG_LABEL } from '@/state/animations';
import { mapCounts, needsLook } from '@/state/retarget';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';

export type MapFilter = 'all' | 'look';

/** The bone map: each humanoid bone, the clip's bone and the model's. */
export function useMapStep() {
  const sheet = useRetargetStore(s => s.sheet);
  const pick = useRetargetStore(s => s.pick);
  const change = useRetargetStore(s => s.change);
  const motion = useMotionStore(s => s.view);
  const joints = motion?.models.find(m => m.asset === sheet?.asset)?.rig?.jointNames ?? [];
  const [filter, setFilter] = useState<MapFilter>('all');
  const plan = sheet?.plan ?? null;
  const map = plan?.map ?? [];
  const missing = map.find(p => !p.target && p.source);
  const near = missing
    ? joints.filter(j => j.toLowerCase().includes(missing.bone.toLowerCase())).slice(0, 3)
    : [];
  return {
    sheet,
    plan,
    /** How far a bone turns to line up, "47°", when both rigs say. */
    turnOf: (bone: string) => {
      const turn = plan?.rest?.turns.find(t => t.bone === bone);
      return turn ? `${Math.round(turn.deg ?? 0)}°` : '';
    },
    /** What to do about the first bone with no match. */
    unmapped: missing
      ? `${missing.bone} has no clear match.${near.length ? ` ${plan?.name ?? 'The model'} has ${near.join(' and ')}.` : ''} Pick one, or leave it unmapped and the bone stays at rest.`
      : null,
    rows: filter === 'look' ? map.filter(needsLook) : map,
    counts: mapCounts(map),
    joints,
    filter,
    setFilter,
    source: plan?.sourceRig ? RIG_LABEL[plan.sourceRig] : 'Mixamo names',
    pick: (bone: string, target: string) => void pick(bone, target || null),
    setSaveMap: (saveMap: boolean) => change({ saveMap }),
  };
}
