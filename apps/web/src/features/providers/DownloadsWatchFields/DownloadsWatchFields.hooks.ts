import { useEffect } from 'react';
import { watchLine } from '@/state/downloads';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The watch switch, its folder and the line about it, read when Settings opens. */
export function useDownloadsWatchFields() {
  const status = useDownloadsStore(s => s.status);
  const saving = useDownloadsStore(s => s.saving);
  const load = useDownloadsStore(s => s.load);
  const setWatch = useDownloadsStore(s => s.setWatch);
  const pickFolder = useDownloadsStore(s => s.pickFolder);
  const home = useAppStore(s => s.boot?.home);
  useEffect(() => {
    void load();
  }, [load]);
  const custom = status?.custom ? (status.folder ?? null) : null;
  return {
    status,
    saving,
    home,
    line: watchLine(status),
    toggle: (on: boolean) => void setWatch(on, custom),
    pickFolder: () => void pickFolder(),
    resetFolder: () => void setWatch(status?.enabled ?? true, null),
  };
}
