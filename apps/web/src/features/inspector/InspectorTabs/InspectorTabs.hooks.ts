import { isGame } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';

/** Which Inspector tab is open, and opening another. A game shows Compare and Budget. */
export function useInspectorTabs() {
  const tab = useInspectorStore(s => s.tab);
  const setTab = useInspectorStore(s => s.setTab);
  const game = useAppStore(s => isGame(s.workspace?.project));
  const shown = game
    ? tab === 'budget'
      ? 'budget'
      : 'compare'
    : tab === 'budget'
      ? 'compare'
      : tab;
  return { tab: shown, setTab, game };
}
