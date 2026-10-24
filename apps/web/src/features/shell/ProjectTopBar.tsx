import { BudgetMeter, Button, StageStepper, TopBar } from '@maquette/ui';
import { useAppStore } from '@/state/app-store';
import { estimatedTransfer, stageViews } from '@/state/pipeline';
import { usePipelineStore } from '@/state/pipeline-store';

/** The stage stepper, the scene budget and Run pipeline, as the Board and the Lab show them. */
export function ProjectTopBar() {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const pipeline = usePipelineStore(s => s.view);
  const runPipeline = usePipelineStore(s => s.run);
  const assets = workspace?.project?.assets ?? [];
  const budget = workspace?.config.budgets.sceneMb ?? 3;
  const latest = pipeline?.runs[0];
  const running = Boolean(pipeline?.active);
  const run = () => {
    void runPipeline();
    setView('pipeline');
  };
  return (
    <TopBar
      lead={<StageStepper stages={stageViews(assets, latest)} />}
      right={
        <>
          <BudgetMeter
            label="Scene budget"
            value={pipeline ? estimatedTransfer(pipeline) / 1e6 : 0}
            target={budget}
            unit="MB"
          />
          {running ? (
            <Button icon="flow-arrow" onClick={() => setView('pipeline')}>
              Run in progress
            </Button>
          ) : (
            <Button
              icon="play"
              disabled={!pipeline || pipeline.def.steps.length === 0}
              title={pipeline ? 'Runs every step, skipping cached ones' : 'Loading the pipeline'}
              onClick={run}
            >
              Run pipeline
            </Button>
          )}
        </>
      }
    />
  );
}
