import { useRenderStore } from '@/state/use-render-store';

/** What the rows show: the plan to pick from, or the render once it runs. */
export function useRenderRows() {
  const plan = useRenderStore(s => s.plan);
  const record = useRenderStore(s => s.record);
  const rows = useRenderStore(s => s.rows);
  const ticked = useRenderStore(s => s.ticked);
  const tick = useRenderStore(s => s.tick);
  const toggleStyle = useRenderStore(s => s.toggleStyle);
  return {
    plan,
    record,
    rows,
    ticked,
    tick,
    toggleStyle,
    shown: (plan?.targets ?? []).filter(t => rows[t.key]),
  };
}
