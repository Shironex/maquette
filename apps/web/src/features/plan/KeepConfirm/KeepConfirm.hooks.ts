import { usePlanLogStore } from '@/state/use-plan-log-store';

/** The Keep waiting for the person, with its two answers. */
export function useKeepConfirm() {
  const ask = usePlanLogStore(s => s.keepAsk);
  const keepAll = usePlanLogStore(s => s.keepAll);
  const cancel = usePlanLogStore(s => s.cancelKeep);
  return {
    needs: ask?.needs ?? null,
    keepAll: () => void keepAll(),
    cancel,
  };
}
