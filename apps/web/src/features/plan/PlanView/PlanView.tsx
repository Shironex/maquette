import { ReactFlowProvider } from '@xyflow/react';
import { Button, EmptyState, Loader, TopBar } from '@maquette/ui';
import { PlanCanvas } from '../PlanCanvas';
import { PlanList } from '../PlanList';
import { usePlanView } from './PlanView.hooks';

/**
 * Plan: a project's planning canvases. The list of canvases, or the one
 * that is open with its notes, pictures and links.
 */
export default function PlanView() {
  const v = usePlanView();
  if (!v.project) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Plan</h1>} />
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="graph"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </>
    );
  }
  if (v.open) {
    return (
      <ReactFlowProvider key={v.open.id}>
        <PlanCanvas />
      </ReactFlowProvider>
    );
  }
  if (!v.list) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Plan</h1>} />
        <main className="grid flex-1 place-items-center" aria-busy={!v.listError}>
          {v.listError ? (
            <EmptyState
              icon="warning-circle"
              title="The canvases could not be read"
              body={v.listError}
              action={
                <Button
                  icon="arrow-clockwise"
                  tooltip="Reads the plans folder again"
                  onClick={v.retry}
                >
                  Try again
                </Button>
              }
            />
          ) : (
            <Loader label="Reading the canvases..." />
          )}
        </main>
      </>
    );
  }
  return <PlanList list={v.list} opening={v.opening} openError={v.openError} />;
}
