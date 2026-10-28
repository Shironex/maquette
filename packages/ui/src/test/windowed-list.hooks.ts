import { useRef } from 'react';
import { useWindowedList } from '../lib/useWindowedList';

/** Pixels each test row is estimated at. */
export const ROW = 40;

/** The test list's scroller and its window. */
export function useList(count: number, columns?: number) {
  const scrollRef = useRef<HTMLDivElement>(null);
  return { scrollRef, w: useWindowedList({ count, scrollRef, estimate: ROW, columns }) };
}
