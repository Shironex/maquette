import { useState } from 'react';
import type { PackRecord } from '@/ipc/client';

/** How many packs show before Show all, so the settings stay in view. */
const FIRST = 2;

/** The packs on show, and the Show all switch once there are more. */
export function useEarlierPacks(recent: readonly PackRecord[]) {
  const [all, setAll] = useState(false);
  return {
    all,
    shown: all ? recent : recent.slice(0, FIRST),
    more: recent.length > FIRST,
    toggle: () => setAll(!all),
  };
}
