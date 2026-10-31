import { isGame } from '@/state/game';
import { inspectorTabs, shownInspectorTab } from '@/state/inspector-tabs';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';

/** Which Inspector tab is open, and opening another. A game has its own tabs. */
export function useInspectorTabs() {
  const tab = useInspectorStore(s => s.tab);
  const setTab = useInspectorStore(s => s.setTab);
  const game = useAppStore(s => isGame(s.workspace?.project));
  return { tab: shownInspectorTab(tab, game), tabs: inspectorTabs(game), setTab, game };
}
