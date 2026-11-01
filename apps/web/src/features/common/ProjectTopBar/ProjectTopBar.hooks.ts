import { nothingToRun } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { estimatedTransfer, stageViews } from '@/state/pipeline';
import { budgetPending } from '@/state/motion';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The stages, the scene budget and the run state the top bar shows. */
export function useProjectTopBar() {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const pipeline = usePipelineStore(s => s.view);
  const runPipeline = usePipelineStore(s => s.run);
  const failed = usePipelineStore(s => s.error !== null);
  const assets = workspace?.project?.assets ?? [];
  const latest = pipeline?.runs[0];
  const nothing = nothingToRun(workspace?.project);
  return {
    stages: stageViews(assets, latest, workspace?.project?.target),
    budget: workspace?.config.budgets.sceneMb ?? 3,
    sceneMb: pipeline ? estimatedTransfer(pipeline) / 1e6 : null,
    budgetPending: budgetPending(Boolean(workspace?.project), failed),
    loaded: Boolean(pipeline),
    canRun: (pipeline?.def.steps.length ?? 0) > 0 && !nothing,
    running: Boolean(pipeline?.active),
    nothing,
    showPipeline: () => setView('pipeline'),
    run: () => {
      void runPipeline();
      setView('pipeline');
    },
  };
}
