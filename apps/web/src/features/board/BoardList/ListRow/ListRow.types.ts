import type { ListRow } from '@/state/board-list';
import type { BoardEnv } from '../../board.types';

export interface ListRowProps {
  row: ListRow;
  board: BoardEnv;
  /** True for the one row that takes the Tab stop. */
  tabStop: boolean;
  /** Opens the row's menu above it, for rows low in the list. */
  menuUp: boolean;
  onToggleMenu: (id: string) => void;
  onFocus: (id: string) => void;
  nowMs: number;
}
