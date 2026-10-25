import { firstViewLine, firstViewMb } from '@/state/layout';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** What the bar shows for the first camera stop, or null before it is measured. */
export function useFirstViewBar() {
  const firstView = usePipelineStore(s => s.view?.firstView);
  if (!firstView) return null;
  return {
    ...firstViewMb(firstView),
    title: firstViewLine(firstView),
    label: firstView.stop ? `First view · ${firstView.stop}` : 'First view · no stop yet',
  };
}
