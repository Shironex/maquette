import type { ClipReview } from '@/ipc/client';
import { canPin } from '@/state/retarget';
import { useRetargetStore } from '@/state/use-retarget-store';

/** What Pin the feet offers for the selected clip, and the numbers once it ran. */
export function usePinFeet(clip: ClipReview) {
  const busy = useRetargetStore(s => s.busy);
  const pinFeet = useRetargetStore(s => s.pinFeet);
  return {
    busy,
    offer: canPin(clip),
    before: clip.unpinnedCm,
    after: clip.slideCm,
    still: clip.status === 'look',
    pin: () => void pinFeet(clip.clip),
  };
}
