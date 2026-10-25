import { useCreditsStore } from '@/state/use-credits-store';

/** Opens the Credits sheet, first doing whatever the caller needs (closing a popover). */
export function useCreditsButton(project: string | null, before?: () => void) {
  const openCredits = useCreditsStore(s => s.openCredits);
  return () => {
    before?.();
    openCredits(project);
  };
}
