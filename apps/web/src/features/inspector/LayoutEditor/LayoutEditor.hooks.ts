import { useEffect, useState } from 'react';
import type { Tool } from '@/state/layout';
import { useAppStore } from '@/state/use-app-store';
import { useLayoutDirty, useLayoutStore } from '@/state/use-layout-store';
import { typing } from '@/lib/typing';

const KEYS: Record<string, Tool> = { w: 'translate', e: 'rotate', r: 'scale' };

/**
 * The editor's store and view state: the grid, the frame counter, how the
 * canvas loaded. Loads the project's layout, and only a new project loads
 * again (agents' saves come as events). Listens for the editor's keys.
 */
export function useLayoutEditor(project: string) {
  const store = useLayoutStore();
  const load = useLayoutStore(s => s.load);
  const setView = useAppStore(s => s.setView);
  const dirty = useLayoutDirty();
  const [grid, setGrid] = useState(true);
  const [frame, setFrame] = useState(0);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    void load(project);
    setReady(false);
    setLoadError(null);
  }, [project, load]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target)) return;
      const s = useLayoutStore.getState();
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === 'z') {
        e.preventDefault();
        if (e.shiftKey) s.redo();
        else s.undo();
      } else if (mod && key === 'y') {
        e.preventDefault();
        s.redo();
      } else if (mod && key === 's') {
        e.preventDefault();
        void s.save();
      } else if (!mod && KEYS[key]) {
        s.setTool(KEYS[key]);
      } else if (!mod && key === 'f') {
        setFrame(n => n + 1);
      } else if (key === 'escape') {
        if (s.preview) s.lookThrough(null);
        else s.select(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return {
    store,
    setView,
    dirty,
    grid,
    toggleGrid: () => setGrid(g => !g),
    frame,
    frameAll: () => setFrame(n => n + 1),
    loadError,
    setLoadError,
    ready,
    setReady,
  };
}
