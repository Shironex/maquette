import { useState } from 'react';
import { useLookAlikesStore } from '@/state/use-look-alikes-store';

/** The look-alikes' scan state, the folded pairs opened so far and Undo. */
export function useLookAlikesView() {
  const { undo, scanning, error } = useLookAlikesStore();
  const [opened, setOpened] = useState<string[]>([]);
  return {
    undo,
    scanning,
    error,
    isOpened: (key: string) => opened.includes(key),
    open: (key: string) => setOpened(o => [...o, key]),
  };
}

/** The side panel's scan and settings. */
export function useLookAlikeSide() {
  const { scanning, scan, saveSettings } = useLookAlikesStore();
  return { scanning, scan, saveSettings };
}
