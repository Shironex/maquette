import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** What the OS calls its file manager. */
export function useFileManager(): string {
  const os = useAppStore(s => s.boot?.os);
  return os === 'windows' ? 'Explorer' : os === 'linux' ? 'the file manager' : 'Finder';
}

/** The handoff's folder, web app and image actions. */
export function useHandoffActions() {
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const openSite = useProvidersStore(s => s.openSite);
  const revealImage = useProvidersStore(s => s.revealImage);
  return { openHandoff, openSite, revealImage };
}
