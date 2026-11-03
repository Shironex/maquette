import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * The open canvas's switches in the top bar: Auto layout after agents (on
 * unless the person turned it off), and on a Game mechanics canvas the
 * Overlaps panel with the count of overlaps not marked as fine.
 */
export function useCanvasSwitches() {
  const auto = usePlansStore(s => s.open?.plan.autoLayout ?? true);
  const mechanics = usePlansStore(s => s.open?.plan.preset === 'mechanics');
  const flagged = usePlansStore(s => (s.open?.overlaps ?? []).filter(o => !o.fine).length);
  const busy = usePlanExtrasStore(s => s.busy);
  const panel = usePlanExtrasStore(s => s.overlaps);
  const setAutoLayout = usePlanExtrasStore(s => s.setAutoLayout);
  const setOverlaps = usePlanExtrasStore(s => s.setOverlaps);
  return {
    auto,
    busy,
    flipAuto: () => void setAutoLayout(!auto),
    overlaps: mechanics ? { flagged, open: panel, toggle: () => setOverlaps(!panel) } : null,
  };
}
