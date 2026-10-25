import { useAppStore } from '@/state/use-app-store';
import { estimatedTransfer, stageViews } from '@/state/pipeline';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The stages, the scene budget and the run state the top bar shows. */
export function useProjectTopBar() {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const pipeline = usePipelineStore(s => s.view);
  const runPipeline = usePipelineStore(s => s.run);
  const assets = workspace?.project?.assets ?? [];
  const latest = pipeline?.runs[0];
  return {
    stages: stageViews(assets, latest),
    budget: workspace?.config.budgets.sceneMb ?? 3,
    sceneMb: pipeline ? estimatedTransfer(pipeline) / 1e6 : 0,
    loaded: Boolean(pipeline),
    canRun: (pipeline?.def.steps.length ?? 0) > 0,
    running: Boolean(pipeline?.active),
    showPipeline: () => setView('pipeline'),
    run: () => {
      void runPipeline();
      setView('pipeline');
    },
  };
}
