import { heldArrivals } from '@/state/downloads';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The downloads waiting for Blender, and the picker that matches them by hand. */
export function useDownloadsOnHold() {
  const held = heldArrivals(useDownloadsStore(s => s.status));
  const openPicker = useDownloadsStore(s => s.openPicker);
  return { held, openPicker };
}
