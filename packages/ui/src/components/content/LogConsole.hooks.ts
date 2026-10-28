import { useEffect, useRef } from 'react';
import { useWindowedList } from '../../lib/useWindowedList';

/** A log line's height before it wraps. */
const LINE = 19;

/**
 * The log body, the window of lines it renders (a long log renders only the
 * lines in view), and the newest line kept in view while lines arrive.
 */
export function useLogLines(autoScroll: boolean | undefined, lines: readonly unknown[]) {
  const body = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const slice = useWindowedList({
    count: lines.length,
    scrollRef: body,
    listRef: list,
    estimate: LINE,
  });
  const { scrollTo } = slice;
  useEffect(() => {
    const el = body.current;
    if (!autoScroll || !el) return;
    el.scrollTop = el.scrollHeight;
    scrollTo(lines.length - 1, 'end');
  }, [autoScroll, lines, scrollTo]);
  return { body, list, slice };
}
