import { useEffect, useRef } from 'react';

/** A ref for the log body that keeps its newest line in view while lines arrive. */
export function useAutoScroll(autoScroll: boolean | undefined, lines: readonly unknown[]) {
  const body = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = body.current;
    if (autoScroll && el) el.scrollTop = el.scrollHeight;
  }, [autoScroll, lines]);
  return body;
}
