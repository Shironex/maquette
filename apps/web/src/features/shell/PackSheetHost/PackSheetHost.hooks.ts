import { usePackStore } from '@/state/use-pack-store';

/** Whether the pack sheet is up, and how it closes. */
export function usePackSheetHost() {
  return { open: usePackStore(s => s.open), close: usePackStore(s => s.close) };
}
