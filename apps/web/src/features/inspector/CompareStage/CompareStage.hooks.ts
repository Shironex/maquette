import { useState } from 'react';
import type { Stop } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';

/** Where the split sits, the view mode, the frame rate and the stop the camera is at. */
export function useCompareStage(stops: Stop[]) {
  const [split, setSplit] = useState(0.5);
  const mode = useInspectorStore(s => s.mode);
  const stop = useInspectorStore(s => s.stop);
  const fps = useInspectorStore(s => s.fps);
  const activeStop = stop ? stops[stop.index] : undefined;
  return { split, setSplit, mode, stop, fps, activeStop };
}
