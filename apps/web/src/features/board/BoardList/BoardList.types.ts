import type { BoardPrefs, ListRow } from '@/state/board-list';
import type { BoardEnv } from '../board.types';

export interface BoardListProps {
  /** The rows to show, filtered and sorted. */
  rows: readonly ListRow[];
  /** How many assets the project has before the filter. */
  total: number;
  prefs: BoardPrefs;
  board: BoardEnv;
}
