import type { DragEvent, MouseEvent } from 'react';
import type { Asset } from '@/ipc/client';
import { cardNumbers } from '@/state/board';
import { useBuiltStore } from '@/state/use-built-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useBudgetCheck } from '@/state/use-budget-check';
import { trianglesLine } from '@/state/game';
import { updateFor } from '@/state/library';
import { useLibraryStore } from '@/state/use-library-store';
import { exportNameOf } from '@/state/providers';
import { assetView } from '../assetView';
import type { BoardEnv } from '../board.types';
import { DRAG_TYPE } from '../constants';

/**
 * What a card shows for its asset: numbers, picture, the latest send, a
 * waiting By hand job and, for a built asset, how it stands against its script.
 */
export function useBoardCard(asset: Asset, board: BoardEnv) {
  const addModel = useProvidersStore(s => s.addModel);
  const built = useBuiltStore(s => s.items.find(b => b.asset === asset.id));
  const rebuild = useBuiltStore(s => s.rebuild);
  const budget = useBudgetCheck(asset);
  const library = useLibraryStore(s => s.view);
  const openUpdate = useLibraryStore(s => s.openUpdate);
  const onDragStart = (e: DragEvent) => {
    e.dataTransfer.setData(DRAG_TYPE, asset.id);
    e.dataTransfer.effectAllowed = 'move';
  };
  const n = cardNumbers(asset);
  const view = assetView(asset, board);
  const card = view.waiting?.card;
  return {
    // A game card reads its triangles against its folder's budget; the size would wrap.
    n: budget ? { ...n, tris: trianglesLine(budget), size: undefined } : n,
    over: budget?.problems.length ? budget.problems.join(', ') : null,
    ...view,
    exportName: card ? exportNameOf(card) : null,
    addModel,
    onDragStart,
    built,
    libraryUpdate: updateFor(library, asset),
    openUpdate: () => {
      if (board.project) void openUpdate({ project: board.project.id, asset: asset.id });
    },
    // A right-click opens the same menu as the dots.
    onContextMenu: (e: MouseEvent) => {
      e.preventDefault();
      board.actions.setCopyFor(null);
      board.actions.setMenuFor(asset.id);
    },
    rebuild: () => {
      if (board.project) void rebuild(board.project.id, asset.id, true);
    },
  };
}
