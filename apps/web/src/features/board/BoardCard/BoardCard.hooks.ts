import type { DragEvent } from 'react';
import type { Asset } from '@/ipc/client';
import { cardNumbers } from '@/state/board';
import { useBuiltStore } from '@/state/use-built-store';
import { useProvidersStore } from '@/state/use-providers-store';
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
  const onDragStart = (e: DragEvent) => {
    e.dataTransfer.setData(DRAG_TYPE, asset.id);
    e.dataTransfer.effectAllowed = 'move';
  };
  return {
    n: cardNumbers(asset),
    ...assetView(asset, board),
    addModel,
    onDragStart,
    built,
    rebuild: () => {
      if (board.project) void rebuild(board.project.id, asset.id, true);
    },
  };
}
