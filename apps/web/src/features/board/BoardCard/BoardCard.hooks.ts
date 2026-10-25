import type { DragEvent } from 'react';
import type { Asset } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { cardNumbers, projectFile } from '@/state/board';
import { cardState, latestJob } from '@/state/providers';
import { useProvidersStore } from '@/state/use-providers-store';
import type { BoardEnv } from '../board.types';
import { DRAG_TYPE } from '../constants';

/** What a card shows for its asset: numbers, picture, the latest send and a waiting By hand job. */
export function useBoardCard(asset: Asset, { workspace, jobs }: BoardEnv) {
  const addModel = useProvidersStore(s => s.addModel);
  const n = cardNumbers(asset);
  const thumbPath = asset.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
  const job = latestJob(jobs, asset.id);
  const sent = cardState(job);
  const waiting = job?.state === 'waiting' ? job : undefined;
  const refPicture = asset.thumbnail?.startsWith('refs/') ? asset.thumbnail : null;
  const onDragStart = (e: DragEvent) => {
    e.dataTransfer.setData(DRAG_TYPE, asset.id);
    e.dataTransfer.effectAllowed = 'move';
  };
  return {
    n,
    thumb: fileUrl(thumbPath) ?? undefined,
    sent,
    waiting,
    refPicture,
    addModel,
    onDragStart,
  };
}
