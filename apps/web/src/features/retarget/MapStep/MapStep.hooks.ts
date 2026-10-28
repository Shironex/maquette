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
  return {
    sheet,
    plan,
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
