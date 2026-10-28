import type { ClipSetting } from '@/ipc/client';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/** Clip settings: each chosen clip with its library clip, one open for editing. */
export function useSettingsStep() {
  const sheet = useRetargetStore(s => s.sheet);
  const change = useRetargetStore(s => s.change);
  const changeSetting = useRetargetStore(s => s.changeSetting);
  const resetNames = useRetargetStore(s => s.resetNames);
  const view = useAnimationsStore(s => s.view);
  const items = view?.items ?? [];
  const rows = (sheet?.settings ?? []).flatMap(setting => {
    const clip = items.find(i => i.clip.id === setting.clip)?.clip;
    return clip ? [{ setting, clip }] : [];
  });
  const selected = rows.find(r => r.setting.clip === sheet?.selected) ?? rows[0] ?? null;
  return {
    sheet,
    rows,
    selected,
    outdated: sheet?.plan?.outdated ?? [],
    select: (clip: string) => change({ selected: clip }),
    edit: (change: Partial<ClipSetting>) => {
      if (selected) changeSetting(selected.setting.clip, change);
    },
    resetNames,
  };
}
