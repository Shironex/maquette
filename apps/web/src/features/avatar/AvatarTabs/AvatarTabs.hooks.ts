import { useAvatarStore } from '@/state/use-avatar-store';

/** The open avatar, for its name and version over the tabs. */
export function useAvatarTabs() {
  return useAvatarStore(s => s.view);
}
