import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * The open canvas's switches in the top bar: Auto layout after agents (on
 * unless the person turned it off), and on a Game mechanics canvas the
 * Overlaps panel with the count of overlaps not marked as fine, or of
 * those marked as fine when every one is.
 */
export function useCanvasSwitches() {
  const auto = usePlansStore(s => s.open?.plan.autoLayout ?? true);
  const mechanics = usePlansStore(s => s.open?.plan.preset === 'mechanics');
  const flagged = usePlansStore(s => (s.open?.overlaps ?? []).filter(o => !o.fine).length);
  const total = usePlansStore(s => s.open?.overlaps?.length ?? 0);
  const busy = usePlanExtrasStore(s => s.busy);
  const panel = usePlanExtrasStore(s => s.overlaps);
  const setAutoLayout = usePlanExtrasStore(s => s.setAutoLayout);
  const setOverlaps = usePlanExtrasStore(s => s.setOverlaps);
  return {
    auto,
    busy,
    flipAuto: () => void setAutoLayout(!auto),
    overlaps: mechanics
      ? {
          // The ones to look at, else the ones marked as fine, so a count shows while any exist.
          count: flagged || total,
          fine: flagged === 0 && total > 0,
          open: panel,
          toggle: () => setOverlaps(!panel),
        }
      : null,
  };
}
