import { useEffect } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { conflictKey, conflictTitle } from '@/state/sync';
import { useAppStore } from '@/state/use-app-store';
import { useFileManager } from '@/state/use-file-manager';
import { CONFLICT_POLL_MS, useSyncConflicts } from '@/state/use-sync-conflicts';

/** The copies to name, looked for on a timer and on focus, and the actions. */
export function useSyncConflictBanner() {
  const conflicts = useSyncConflicts(s => s.conflicts);
  const dismissed = useSyncConflicts(s => s.dismissed);
  const refresh = useSyncConflicts(s => s.refresh);
  const dismiss = useSyncConflicts(s => s.dismiss);
  const root = useAppStore(s => s.workspace?.root);
  const notify = useAppStore(s => s.notify);
  const { reveal } = useFileManager();

  useEffect(() => {
    if (!root) return;
    void refresh();
    const timer = window.setInterval(() => void refresh(), CONFLICT_POLL_MS);
    const onFocus = () => void refresh();
    window.addEventListener('focus', onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('focus', onFocus);
    };
  }, [root, refresh]);

  const shown = conflicts.length > 0 && dismissed !== conflictKey(conflicts);
  return {
    shown,
    conflicts,
    title: conflictTitle(conflicts),
    revealLabel: reveal('copy'),
    dismiss,
    show: async (path: string) => {
      try {
        await commands.workspaceReveal(path);
      } catch (error) {
        notify({ status: 'failed', title: 'Could not show the copy', detail: errorMessage(error) });
      }
    },
  };
}
