import { useInspectorStore } from '@/state/use-inspector-store';

/** Which Inspector tab is open, and opening another. */
export function useInspectorTabs() {
  const tab = useInspectorStore(s => s.tab);
  const setTab = useInspectorStore(s => s.setTab);
  return { tab, setTab };
}
