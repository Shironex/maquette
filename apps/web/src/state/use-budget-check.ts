import type { Asset, BudgetCheck } from '@/ipc/client';
import { checkAsset, gameOf } from './game';
import { useAppStore } from './use-app-store';

/** The asset's model against its folder's budget, in a game project; else null. */
export function useBudgetCheck(asset: Asset): BudgetCheck | null {
  const game = useAppStore(s => gameOf(s.workspace?.project));
  return game ? checkAsset(game, asset) : null;
}

/** True while the open project is a game: its views drop the website's scene budget. */
export function useIsGame(): boolean {
  return useAppStore(s => gameOf(s.workspace?.project) !== null);
}
