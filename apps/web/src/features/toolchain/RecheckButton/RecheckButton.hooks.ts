import { useAppStore } from '@/state/use-app-store';

export function useRecheckButton() {
  const checking = useAppStore(s => s.tools.checking);
  const detect = useAppStore(s => s.detectTools);
  return { checking, detect };
}
