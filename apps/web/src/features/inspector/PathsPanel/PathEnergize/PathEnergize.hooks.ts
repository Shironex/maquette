import { useAppStore } from '@/state/use-app-store';

/** Opening the Timeline view, where the energize keys are edited. */
export function usePathEnergize() {
  const setView = useAppStore(s => s.setView);
  return { openTimeline: () => setView('timeline') };
}
