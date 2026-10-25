import { useInspectorStore } from '@/state/use-inspector-store';

/** The stop the camera is at, and flying to another. */
export function useCameraStops() {
  const stop = useInspectorStore(s => s.stop);
  const flyTo = useInspectorStore(s => s.flyTo);
  return { stop, flyTo };
}
