import { usePlanCameraStore } from '@/state/use-plan-camera-store';

/** Compare after `layout.json` changed: both sides, and Rebase or Start over. */
export function useCameraCompare() {
  const ask = usePlanCameraStore(s => s.compare);
  const outside = usePlanCameraStore(s => s.view?.outside ?? null);
  const rebase = usePlanCameraStore(s => s.rebase);
  const close = usePlanCameraStore(s => s.closeCompare);
  const again = usePlanCameraStore(s => s.askCompare);
  return {
    report: ask?.report ?? null,
    busy: ask?.busy ?? false,
    /** Why the rebase was refused: layout.json changed again since this comparison. */
    error: ask?.error ?? null,
    reviewAgain: () => void again(),
    outside,
    rebase: () => void rebase('rebase'),
    startOver: () => void rebase('start-over'),
    close,
  };
}
