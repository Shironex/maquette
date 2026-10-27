import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useUpdaterStore } from '@/state/use-updater-store';
import { phaseUpdate, showsPill } from '@/state/updater';

/** Follows the shell's update steps while the app is open. */
export function useUpdaterFeed() {
  const start = useUpdaterStore(s => s.start);
  useEffect(() => start(), [start]);
}

/** Whether the pill shows, its version, and the click that opens About. */
export function useUpdatePill() {
  useUpdaterFeed();
  const mode = useUpdaterStore(s => s.mode);
  const phase = useUpdaterStore(s => s.phase);
  const reveal = useUpdaterStore(s => s.reveal);
  const setView = useAppStore(s => s.setView);
  const update = phaseUpdate(phase);
  return {
    shown: showsPill({ mode, phase }),
    version: update?.version ?? '',
    ready: phase.kind === 'ready',
    open: () => {
      setView('settings');
      reveal();
    },
  };
}
