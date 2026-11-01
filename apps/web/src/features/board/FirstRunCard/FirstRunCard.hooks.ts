import { isGame } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { useGameStore } from '@/state/use-game-store';
import { GAME_STEPS, STEPS } from './steps';

/** Whether the tips were hidden, remembered across launches, and where they lead. */
export function useFirstRunCard() {
  const setView = useAppStore(s => s.setView);
  const project = useAppStore(s => s.workspace?.project);
  const openImport = useGameStore(s => s.openImport);
  const game = isGame(project);
  const { hidden, hide } = useFirstRunTips();
  return {
    hidden,
    hide,
    setView,
    steps: game ? GAME_STEPS : STEPS,
    /** A game imports from its repo; a website takes models in through the Inbox. */
    importModels: () => {
      if (game && project) void openImport(project.id);
      else setView('inbox');
    },
  };
}
