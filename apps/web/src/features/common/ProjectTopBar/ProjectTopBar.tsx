import { Button, StageStepper, TopBar } from '@maquette/ui';
import { SceneBudgetMeter } from '../SceneBudgetMeter';
import { useProjectTopBar } from './ProjectTopBar.hooks';
import type { ProjectTopBarProps } from './ProjectTopBar.types';

/**
 * The stage stepper, the scene budget and Run pipeline, as the Board and
 * the Lab show them; `extra` goes before Run pipeline.
 */
export default function ProjectTopBar({ extra }: ProjectTopBarProps) {
  const { stages, budget, sceneMb, loaded, canRun, running, showPipeline, run } =
    useProjectTopBar();
  return (
    <TopBar
      lead={<StageStepper stages={stages} />}
      right={
        <>
          <SceneBudgetMeter value={sceneMb} target={budget} />
          {extra}
          {running ? (
            <Button icon="flow-arrow" onClick={showPipeline}>
              Run in progress
            </Button>
          ) : (
            <Button
              icon="play"
              disabled={!canRun}
              disabledReason={loaded ? undefined : 'Loading the pipeline'}
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
