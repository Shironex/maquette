import type { Asset } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { cardState, latestJob } from '@/state/providers';
import type { BoardEnv } from './board.types';

/**
 * What a card and a list row show for an asset besides its numbers: the
 * picture, the latest send and a By hand job waiting for its model.
 */
export function assetView(asset: Asset, { workspace, jobs }: Pick<BoardEnv, 'workspace' | 'jobs'>) {
  const thumbPath = asset.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
  const job = latestJob(jobs, asset.id);
  return {
    thumb: fileUrl(thumbPath) ?? undefined,
    sent: cardState(job),
    waiting: job?.state === 'waiting' ? job : undefined,
    refPicture: asset.thumbnail?.startsWith('refs/') ? asset.thumbnail : null,
  };
}
