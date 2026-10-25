import type { Asset } from '@/ipc/client';
import type { BoardEnv } from '../board.types';

export interface BoardCardProps {
  asset: Asset;
  board: BoardEnv;
}
