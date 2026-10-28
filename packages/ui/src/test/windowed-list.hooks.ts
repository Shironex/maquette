import { useRef } from 'react';
import { useGridColumns, useWindowedList } from '../lib/useWindowedList';

/** Pixels each test row is estimated at. */
export const ROW = 40;

/** The test list's scroller and its window. */
export function useList(count: number, columns?: number) {
  const scrollRef = useRef<HTMLDivElement>(null);
  return { scrollRef, w: useWindowedList({ count, scrollRef, estimate: ROW, columns }) };
}

/** A test grid's list and the columns that fit across it. */
export function useGrid() {
  const listRef = useRef<HTMLUListElement>(null);
  return { listRef, columns: useGridColumns(listRef, 200, 12) };
}
