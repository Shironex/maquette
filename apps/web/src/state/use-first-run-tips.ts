/** Whether the empty Board's first run tips were hidden, remembered across launches. */
import { create } from 'zustand';
import { readFlag, writeFlag } from '@/lib/storage';

const TIPS_KEY = 'maquette.board.tipsHidden';

export const useFirstRunTips = create<{ hidden: boolean; hide: () => void }>(set => ({
  hidden: readFlag(TIPS_KEY),
  hide: () => {
    writeFlag(TIPS_KEY, true);
    set({ hidden: true });
  },
}));
