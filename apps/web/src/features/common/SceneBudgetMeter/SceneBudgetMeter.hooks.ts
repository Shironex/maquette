import { firstViewLine } from '@/state/layout';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The tooltip line: the whole scene, then what the first camera stop loads. */
export function useSceneBudgetLabel(value: number, target: number) {
  const firstView = usePipelineStore(s => s.view?.firstView);
  return `The whole scene: ${value.toFixed(1)} of ${target} MB. ${firstViewLine(firstView)}`;
}
