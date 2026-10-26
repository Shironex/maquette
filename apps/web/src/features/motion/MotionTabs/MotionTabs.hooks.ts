import type { MotionTab } from '@/state/motion';
import { useMotionStore } from '@/state/use-motion-store';

/** The selected tab and switching it. */
export function useMotionTabs() {
  const tab = useMotionStore(s => s.tab);
  const setTab = useMotionStore(s => s.setTab);
  return { tab, setTab: (id: string) => setTab(id as MotionTab) };
}
