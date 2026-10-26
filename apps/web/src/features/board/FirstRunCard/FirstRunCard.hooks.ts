import { useState } from 'react';
import { readFlag, writeFlag } from '@/lib/storage';
import { isGame } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { GAME_STEPS, STEPS } from './steps';

const TIPS_KEY = 'maquette.board.tipsHidden';

/** Whether the tips were hidden, remembered across launches, and where they lead. */
export function useFirstRunCard() {
  const setView = useAppStore(s => s.setView);
  const project = useAppStore(s => s.workspace?.project);
  const openImport = useGameStore(s => s.openImport);
  const game = isGame(project);
  const [hidden, setHidden] = useState(() => readFlag(TIPS_KEY));
  const hide = () => {
    setHidden(true);
    writeFlag(TIPS_KEY, true);
  };
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
