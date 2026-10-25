import { useState } from 'react';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAppStore } from '@/state/use-app-store';

const TIPS_KEY = 'maquette.board.tipsHidden';

/** Whether the tips were hidden, remembered across launches, and where they lead. */
export function useFirstRunCard() {
  const setView = useAppStore(s => s.setView);
  const [hidden, setHidden] = useState(() => readFlag(TIPS_KEY));
  const hide = () => {
    setHidden(true);
    writeFlag(TIPS_KEY, true);
  };
  return { hidden, hide, setView };
}
