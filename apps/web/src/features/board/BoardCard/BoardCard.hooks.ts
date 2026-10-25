import type { DragEvent } from 'react';
import type { Asset } from '@/ipc/client';
import { cardNumbers } from '@/state/board';
import { useProvidersStore } from '@/state/use-providers-store';
import { assetView } from '../assetView';
import type { BoardEnv } from '../board.types';
import { DRAG_TYPE } from '../constants';

/** What a card shows for its asset: numbers, picture, the latest send and a waiting By hand job. */
export function useBoardCard(asset: Asset, board: BoardEnv) {
  const addModel = useProvidersStore(s => s.addModel);
  const onDragStart = (e: DragEvent) => {
    e.dataTransfer.setData(DRAG_TYPE, asset.id);
    e.dataTransfer.effectAllowed = 'move';
  };
  return { n: cardNumbers(asset), ...assetView(asset, board), addModel, onDragStart };
}
