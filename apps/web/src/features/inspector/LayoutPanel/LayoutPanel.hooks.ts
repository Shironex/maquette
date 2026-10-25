import { useLayoutStore } from '@/state/use-layout-store';

/** The layout store the panel lists and edits. */
export function useLayoutPanel() {
  const store = useLayoutStore();
  return { store };
}
