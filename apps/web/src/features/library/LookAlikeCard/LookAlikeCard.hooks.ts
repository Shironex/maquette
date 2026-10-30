import { useState } from 'react';
import type { LibraryItem, LookAlikePair } from '@/ipc/client';
import { pairKey, useLookAlikesStore } from '@/state/use-look-alikes-store';

/** A pair's actions and, for a same name pair, the two names being typed. */
export function useLookAlikeCard(pair: LookAlikePair, a: LibraryItem, b: LibraryItem) {
  const { busy, decide, merge, renameBoth } = useLookAlikesStore();
  const [first, setFirst] = useState(a.entry.name);
  const [second, setSecond] = useState(b.entry.name);
  const working = busy === pairKey(pair);
  const clash =
    !first.trim() || !second.trim() || first.trim().toLowerCase() === second.trim().toLowerCase();
  return {
    working,
    first,
    second,
    setFirst,
    setSecond,
    clash,
    merge: () => void merge(pair),
    keepBoth: () => void decide(pair, 'keep-both'),
    notAlike: () => void decide(pair, 'not-alike'),
    keepNames: () => void decide(pair, 'keep-names'),
    renameBoth: () => void renameBoth(pair, first.trim(), second.trim()),
  };
}
