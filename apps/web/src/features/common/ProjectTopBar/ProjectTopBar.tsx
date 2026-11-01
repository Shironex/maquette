import { Button, StageStepper, TopBar } from '@maquette/ui';
import { SceneBudgetMeter } from '../SceneBudgetMeter';
import { useProjectTopBar } from './ProjectTopBar.hooks';
import type { ProjectTopBarProps } from './ProjectTopBar.types';

/**
 * The stage stepper, the scene budget and Run pipeline, as the Board and
 * the Lab show them; `extra` goes before Run pipeline.
 */
export default function ProjectTopBar({ extra, primaryRun }: ProjectTopBarProps) {
  const {
    stages,
    budget,
    sceneMb,
    budgetPending,
    loaded,
    canRun,
    running,
    nothing,
    showPipeline,
    run,
  } = useProjectTopBar();
  return (
    <TopBar
      lead={<StageStepper stages={stages} />}
      right={
        <>
          <SceneBudgetMeter value={sceneMb} target={budget} pending={budgetPending} />
          {extra}
          {running ? (
            <Button
              icon="flow-arrow"
              tooltip="Goes to the Pipeline to watch the run"
              onClick={showPipeline}
            >
              Run in progress
            </Button>
          ) : (
            <Button
              variant={primaryRun ? 'primary' : 'secondary'}
              icon="play"
              disabled={!canRun}
              disabledReason={loaded ? (nothing ?? undefined) : 'Loading the pipeline'}
              tooltip="Runs every step, skipping cached ones"
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
