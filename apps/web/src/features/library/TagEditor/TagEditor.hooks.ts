import { useState, type KeyboardEvent } from 'react';
import type { LibraryEntry } from '@/ipc/client';
import { tagEntries } from '@/state/library-actions';
import { cleanTag } from '@/state/library-selection';

/** One model's tag field and the change on its way. */
export function useTagEditor(entry: LibraryEntry) {
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const change = async (add: string[], remove: string[]) => {
    setBusy(true);
    const ok = await tagEntries([entry.id], add, remove);
    setBusy(false);
    if (ok && add.length) setTyped('');
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && cleanTag(typed)) void change([cleanTag(typed)], []);
  };
  return { typed, setTyped, busy, change, onKeyDown };
}
