import type { Asset, Stage } from '@/ipc/client';
import type { BoardEnv } from '../board.types';

export interface BoardColumnProps {
  stage: Stage;
  /** The cards in this column. */
  assets: readonly Asset[];
  /** The whole Board has no card yet, so Concept shows the drop zone. */
  boardEmpty: boolean;
  board: BoardEnv;
}
