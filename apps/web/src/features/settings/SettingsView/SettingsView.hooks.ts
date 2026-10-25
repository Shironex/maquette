import { useAppStore } from '@/state/use-app-store';

/** The open workspace's root, the key Settings remounts on. */
export function useSettingsRoot() {
  return useAppStore(s => s.workspace?.root);
}
