import { useAnimationsStore } from '@/state/use-animations-store';

/** The open Save clips sheet: the model's clips, the chosen ones and what they weigh. */
export function useSaveClipsSheet() {
  const sheet = useAnimationsStore(s => s.saving);
  const busy = useAnimationsStore(s => s.busy);
  const items = useAnimationsStore(s => s.view?.items);
  const { closeSave, changeSave, runSave } = useAnimationsStore.getState();
  const plan = sheet?.plan ?? null;
  const chosen = sheet?.chosen ?? [];
  const n = chosen.length;
  const pack = sheet?.pack.trim() ?? '';
  return {
    sheet,
    plan,
    busy,
    chosen,
    count: n,
    clips: `${n} ${n === 1 ? 'clip' : 'clips'}`,
    bytes: (plan?.clips ?? [])
      .filter(c => chosen.includes(c.name))
      .reduce((sum, c) => sum + (c.bytes ?? 0), 0),
    newPack: pack !== '' && !(items ?? []).some(i => i.clip.pack === pack),
    meta: plan
      ? `From ${plan.name}${plan.version ? ` v${plan.version}` : ''} · ${plan.file}`
      : 'Reading the model...',
    toggle: (name: string, on: boolean) =>
      changeSave({ chosen: on ? [...chosen, name] : chosen.filter(c => c !== name) }),
    change: changeSave,
    close: closeSave,
    run: () => void runSave(),
  };
}
