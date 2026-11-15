import { useState } from 'react';
import { readFlag, writeFlag } from './storage';

/** A flag this viewer's browser remembers under `key` (a folded panel), off when storage is missing. */
export function useRememberedFlag(key: string): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(() => readFlag(key));
  return [
    on,
    (next: boolean) => {
      setOn(next);
      writeFlag(key, next);
    },
  ];
}
