import { budgetSummary } from '@/state/game';
import { firstViewLine } from '@/state/layout';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The tooltip line: the whole scene, then what the first camera stop loads. */
export function useSceneBudgetLabel(value: number | null, target: number) {
  const firstView = usePipelineStore(s => s.view?.firstView);
  if (value === null) return 'The scene budget is read with the pipeline.';
  return `The whole scene: ${value.toFixed(1)} of ${target} MB. ${firstViewLine(firstView)}`;
}

/** A game project's models against their budgets; null for a website. */
export function useModelBudget() {
  return budgetSummary(useAppStore(s => s.workspace?.project));
}
