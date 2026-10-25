import type { BoardPrefs } from '@/state/board-list';

export interface ListToolbarProps {
  prefs: BoardPrefs;
  /** Rows after the filter, and assets in the project. */
  shown: number;
  total: number;
  onChange: (change: Partial<BoardPrefs>) => void;
}
