import { useEffect } from 'react';
import { filterClips, NO_ANIMATION_FILTERS, packCounts, tagCounts } from '@/state/animations';
import { chosenTotals, isHumanoidClip, riggedModels } from '@/state/retarget';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/** Choose clips: the project's rigged models, the library filtered, what the chosen add. */
export function useChooseStep() {
  const sheet = useRetargetStore(s => s.sheet);
  const change = useRetargetStore(s => s.change);
  const toggle = useRetargetStore(s => s.toggle);
  const setAsset = useRetargetStore(s => s.setAsset);
  const view = useAnimationsStore(s => s.view);
  const load = useAnimationsStore(s => s.load);
  const motion = useMotionStore(s => s.view);
  useEffect(() => {
    if (!view) void load();
  }, [view, load]);
  const items = view?.items ?? [];
  const models = motion && motion.project === sheet?.project ? riggedModels(motion.models) : [];
  const humanoid = items.filter(i => isHumanoidClip(i.clip));
  const shown = filterClips(sheet?.humanoidOnly ? humanoid : items, {
    ...NO_ANIMATION_FILTERS,
    query: sheet?.query ?? '',
    pack: sheet?.pack ?? null,
    tag: sheet?.tag ?? null,
  });
  // The clip last ticked, which is the one the preview plays.
  const preview =
    items.find(i => i.clip.id === sheet?.selected && sheet.chosen.includes(i.clip.id)) ??
    items.find(i => i.clip.id === sheet?.chosen[0]) ??
    null;
  const model = models.find(m => m.asset === sheet?.asset) ?? null;
  return {
    sheet,
    models,
    model,
    shown,
    packs: packCounts(items),
    tags: tagCounts(items),
    humanoid: humanoid.length,
    previewClip: preview
      ? { id: preview.clip.id, name: preview.clip.name, file: preview.file }
      : null,
    total: items.length,
    totals: chosenTotals(items, sheet?.chosen ?? []),
    change,
    toggle,
    setAsset: (asset: string) => void setAsset(asset),
  };
}
