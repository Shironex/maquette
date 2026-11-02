import { clock } from '@/lib/clock';
import { whenOf } from '@/state/plan-log';
import { usePlanLogStore } from '@/state/use-plan-log-store';

/** The Keep waiting for the person, with its two answers; each change shows its time and id. */
export function useKeepConfirm() {
  const ask = usePlanLogStore(s => s.keepAsk);
  const keepAll = usePlanLogStore(s => s.keepAll);
  const cancel = usePlanLogStore(s => s.cancelKeep);
  return {
    needs: ask?.needs ?? null,
    keepAll: () => void keepAll(),
    cancel,
    when: (at: string) => whenOf(at, clock.now()),
  };
}
