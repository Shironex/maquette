import { useAvatarStore } from '@/state/use-avatar-store';

/** The open avatar, for its readiness. */
export function useReadinessPanel() {
  return useAvatarStore(s => s.view);
}
