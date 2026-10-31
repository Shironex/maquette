import { useEffect, useMemo, useState } from 'react';
import { fromLayout } from '@/state/layout';
import { addStroke } from '@/state/path-stroke';
import type { Vec3 } from '@/state/inspector';
import {
  addPath,
  addPoint,
  movePoint,
  removePoint,
  snapPoint,
  type PathSelection,
} from '@/state/paths';
import { usePathsStore } from '@/state/use-paths-store';
import { typing } from '@/lib/typing';
import type { PathsCanvasHandlers } from '../PathsCanvas';

/** Where a new path starts and ends: a metre across the middle of the scene. */
const NEW_FROM: Vec3 = [-0.5, 0, 0];
const NEW_TO: Vec3 = [0.5, 0, 0];

/** Adds a path and selects it. */
export function newPath(): void {
  const s = usePathsStore.getState();
  const { draft, id } = addPath(s.draft, NEW_FROM, NEW_TO);
  s.edit(() => draft, { path: id, point: null });
}

/** Removes the selected point, when its path keeps two. */
export function removeSelectedPoint(): void {
  const s = usePathsStore.getState();
  const sel = s.selection;
  const path = sel ? s.draft.paths.find(p => p.id === sel.path) : undefined;
  if (!sel || sel.point === null || !path || path.points.length <= 2) return;
  s.edit(d => removePoint(d, sel.path, sel.point!), { path: sel.path, point: null });
}

/**
 * The editor's store and view state: the frame counter and how the canvas
 * loaded, the canvas's handlers and the keys. Loads the project's layout
 * on mount, so it shows what the other editors saved.
 */
export function usePathsEditor(project: string) {
  const store = usePathsStore();
  const load = usePathsStore(s => s.load);
  const [frame, setFrame] = useState(0);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const layout = useMemo(() => (store.view ? fromLayout(store.view.layout) : null), [store.view]);

  useEffect(() => {
    void load(project);
    setReady(false);
    setLoadError(null);
  }, [project, load]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target)) return;
      const s = usePathsStore.getState();
      if (e.metaKey || e.ctrlKey) {
        if (e.key.toLowerCase() === 's') {
          e.preventDefault();
          void s.save();
        }
        return;
      }
      const key = e.key.toLowerCase();
      if (key === 'v') s.setTool('select');
      else if (key === 'a' && s.selection) s.setTool('add');
      else if (key === 'd' && s.selection) s.setTool('surface');
      else if (key === 'f') setFrame(n => n + 1);
      else if (key === 'delete' || key === 'backspace') removeSelectedPoint();
      else if (key === 'escape') {
        if (s.tool !== 'select') s.setTool('select');
        else if (s.selection?.point !== null && s.selection) {
          s.select({ path: s.selection.path, point: null });
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const handlers = useMemo<PathsCanvasHandlers>(
    () => ({
      onSelect: (selection: PathSelection) => usePathsStore.getState().select(selection),
      onAddPoint: point => {
        const s = usePathsStore.getState();
        const sel = s.selection;
        if (!sel) return;
        const { draft, index } = addPoint(s.draft, sel.path, point, sel.point);
        if (index >= 0) s.edit(() => draft, { path: sel.path, point: index });
      },
      onStroke: stroke => {
        const s = usePathsStore.getState();
        const sel = s.selection;
        if (!sel) return;
        const { draft, index } = addStroke(s.draft, sel.path, stroke, sel.point, s.snap);
        if (index >= 0) s.edit(() => draft, { path: sel.path, point: index });
      },
      onDragPoint: (index, point) => {
        const s = usePathsStore.getState();
        const id = s.selection?.path;
        if (id) s.preview(d => movePoint(d, id, index, snapPoint(point, false)));
      },
      onDropPoint: (index, point) => {
        const s = usePathsStore.getState();
        const id = s.selection?.path;
        if (id) s.edit(d => movePoint(d, id, index, snapPoint(point, false)));
      },
      onError: setLoadError,
      onReady: () => setReady(true),
    }),
    []
  );

  return {
    store,
    layout,
    frame,
    frameAll: () => setFrame(n => n + 1),
    handlers,
    loadError,
    ready,
  };
}
