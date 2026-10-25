import type { PlainStop } from '@/state/layout';
import { useLayoutStore } from '@/state/use-layout-store';

/** The layout store the stop's fields edit, and whether the view looks through the stop. */
export function useStopDetails(stop: PlainStop) {
  const store = useLayoutStore();
  const looking = store.preview?.id === stop.id;
  return { store, looking };
}
