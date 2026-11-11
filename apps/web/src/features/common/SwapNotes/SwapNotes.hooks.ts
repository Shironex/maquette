import { useEffect, useState } from 'react';
import { swapLines } from '@/state/built-replace';

/**
 * One line per model a built asset replaced since the last Ship, read again
 * whenever `refresh` changes (a run that ended, a plan that reloaded).
 */
export function useSwapNotes(project: string, refresh: string | number | null = null): string[] {
  const [found, setFound] = useState<{ project: string; lines: string[] }>({
    project,
    lines: [],
  });
  useEffect(() => {
    let live = true;
    void swapLines(project).then(lines => {
      if (live) setFound({ project, lines });
    });
    return () => {
      live = false;
    };
  }, [project, refresh]);
  // Lines read for another project are not this one's.
  return found.project === project ? found.lines : [];
}
