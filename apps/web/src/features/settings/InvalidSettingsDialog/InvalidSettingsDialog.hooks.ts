import type { ViewId } from '@/state/views';
import { useAppStore } from '@/state/use-app-store';
import { useSettingsGuard } from '@/state/settings-guard';

/** The two answers: stay in Settings, or drop the invalid fields and leave. */
export function useInvalidSettingsDialog(leaveTo: ViewId, discardInvalid: () => void) {
  const stay = () => useSettingsGuard.getState().stay();
  const discard = () => {
    discardInvalid();
    useAppStore.getState().setView(leaveTo, true);
  };
  return { stay, discard };
}
