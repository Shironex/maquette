import { heldArrivals } from '@/state/downloads';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** Whether Blender is known to be missing, and how many downloads wait for it. */
export function useBlenderBanner() {
  const { reports, checking } = useAppStore(s => s.tools);
  const setView = useAppStore(s => s.setView);
  const held = heldArrivals(useDownloadsStore(s => s.status)).length;
  const blender = reports.find(r => r.id === 'blender');
  return {
    missing: !checking && blender !== undefined && blender.status !== 'found',
    held,
    openSettings: () => setView('settings'),
  };
}
