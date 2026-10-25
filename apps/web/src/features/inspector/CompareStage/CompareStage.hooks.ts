import { useMemo, useState } from 'react';
import type { Stop } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useMaterialsStore } from '@/state/use-materials-store';

/**
 * Where the split sits, the view mode, the frame rate, the stop the camera
 * is at, and the project's material tweaks by asset, edits over what is saved.
 */
export function useCompareStage(stops: Stop[], project: string) {
  const [split, setSplit] = useState(0.5);
  const mode = useInspectorStore(s => s.mode);
  const stop = useInspectorStore(s => s.stop);
  const fps = useInspectorStore(s => s.fps);
  const activeStop = stop ? stops[stop.index] : undefined;
  const saved = useMaterialsStore(s => s.saved[project]);
  const edits = useMaterialsStore(s => s.edits[project]);
  const looks = useMemo(() => ({ ...saved?.models, ...edits }), [saved, edits]);
  return { split, setSplit, mode, stop, fps, activeStop, looks };
}
