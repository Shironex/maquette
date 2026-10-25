import { Button, StageStepper, TopBar } from '@maquette/ui';
import { SceneBudgetMeter } from '../SceneBudgetMeter';
import { useProjectTopBar } from './ProjectTopBar.hooks';

/** The stage stepper, the scene budget and Run pipeline, as the Board and the Lab show them. */
export default function ProjectTopBar() {
  const { stages, budget, sceneMb, loaded, canRun, running, showPipeline, run } =
    useProjectTopBar();
  return (
    <TopBar
      lead={<StageStepper stages={stages} />}
      right={
        <>
          <SceneBudgetMeter value={sceneMb} target={budget} />
          {running ? (
            <Button icon="flow-arrow" onClick={showPipeline}>
              Run in progress
            </Button>
          ) : (
            <Button
              icon="play"
              disabled={!canRun}
              title={loaded ? 'Runs every step, skipping cached ones' : 'Loading the pipeline'}
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
