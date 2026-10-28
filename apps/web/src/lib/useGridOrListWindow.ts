import { useRef } from 'react';
import { useGridColumns, useWindowedList } from '@maquette/ui';

/** A library's card and row heights before they are measured, and its narrowest grid column. */
export interface GridOrListSizes {
  card: number;
  row: number;
  /** The `minmax()` minimum of the grid's columns. */
  column: number;
}

/** The gaps the libraries' grids (`gap-3`) and lists (`gap-1`) use. */
const GRID_GAP = 12;
const LIST_GAP = 4;

/**
 * A library's scroller and the cards or rows in view, windowed once it is
 * long: the Models and Animations libraries share it.
 */
export function useGridOrListWindow(count: number, grid: boolean, sizes: GridOrListSizes) {
  const scrollRef = useRef<HTMLElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const columns = useGridColumns(listRef, sizes.column, GRID_GAP);
  const shown = useWindowedList({
    count,
    scrollRef,
    listRef,
    estimate: grid ? sizes.card : sizes.row,
    gap: grid ? GRID_GAP : LIST_GAP,
    columns: grid ? columns : 1,
  });
  return { scrollRef, listRef, shown };
}
