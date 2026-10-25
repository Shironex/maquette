import { useEffect, useState } from 'react';

/**
 * What `load(arg)` resolves to, asked again a moment after `arg` changes
 * (compared by value, so a fresh but equal object does not ask twice).
 * Keeps the last answer while the next one is on its way; null before the
 * first, after a failure, or while `arg` is null. `load` must be stable.
 */
export function useAsyncValue<A, T>(
  load: (arg: A) => Promise<T>,
  arg: A | null,
  delayMs = 150
): T | null {
  const key = arg === null ? null : JSON.stringify(arg);
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    if (key === null) {
      setValue(null);
      return;
    }
    let live = true;
    const timer = window.setTimeout(() => {
      load(JSON.parse(key) as A).then(
        next => {
          if (live) setValue(next);
        },
        () => {
          if (live) setValue(null);
        }
      );
    }, delayMs);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [load, key, delayMs]);
  return value;
}
