import { useEffect, useState } from 'react';
import { clock } from '@/lib/clock';

/** The current time, refreshed every `intervalMs`, for "4 s ago" labels. */
export function useNow(intervalMs = 1000): number {
  const [now, setNow] = useState(() => clock.nowMs());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(clock.nowMs()), intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs]);
  return now;
}
