import { useRef, useState } from 'react';
import type { BoneMap } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useTopEscape } from '@/lib/useTopEscape';
import { useMotionStore } from '@/state/use-motion-store';

export type MatchFilter = 'all' | 'missing';

export function mapCounts(map: readonly BoneMap[]) {
  const exact = map.filter(b => b.how === 'exact').length;
  const byName = map.filter(b => b.how === 'by-name').length;
  return { matched: exact + byName, missing: map.length - exact - byName };
}

/** The bone map of the selected model, open from Map bones or Retarget a clip. */
export function useBoneMapDialog() {
  const open = useMotionStore(s => s.boneMap);
  const setBoneMap = useMotionStore(s => s.setBoneMap);
  const model = useMotionStore(s => s.view?.models.find(m => m.asset === s.asset) ?? null);
  const [filter, setFilter] = useState<MatchFilter>('all');
  const ref = useRef<HTMLDivElement>(null);
  const close = () => setBoneMap(false);
  useTopEscape(ref, close);
  const map = model?.rig?.map ?? [];
  const rows = filter === 'missing' ? map.filter(b => b.how === 'missing') : map;
  return {
    open: open && Boolean(model?.rig),
    model,
    rows,
    counts: mapCounts(map),
    filter,
    setFilter,
    url: model ? fileUrl(model.file) : null,
    ref,
    close,
  };
}
