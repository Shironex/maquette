import { useEffect, useRef } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';

/**
 * Closes the sheet when another project opens under it: the switcher's
 * shortcut still works there. Lives here so it holds while the sheet's code
 * loads or after it failed. Packs are workspace wide, so one of another
 * project, opened or resumed here, keeps it open. A pack left behind goes on,
 * and one that then needs you is answered from Earlier packs.
 */
function useClosesWithProject() {
  const current = useAppStore(s => s.workspace?.project?.id);
  const open = usePackStore(s => s.open);
  const close = usePackStore(s => s.close);
  const previous = useRef(current);
  useEffect(() => {
    if (previous.current === current) return;
    previous.current = current;
    if (open) close();
  }, [current, open, close]);
}

/** Whether the pack sheet is up, how it closes, and whether a pack is on show. */
export function usePackSheetHost() {
  useClosesWithProject();
  return {
    open: usePackStore(s => s.open),
    close: usePackStore(s => s.close),
    hasPack: usePackStore(s => s.record !== null),
  };
}
