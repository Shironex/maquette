import { Button, EmptyState, Loader } from '@maquette/ui';
import { Inspector } from '../Inspector';
import { InspectorShell } from '../InspectorShell';
import { InspectorTabs } from '../InspectorTabs';
import { LayoutBarActions } from '../LayoutBarActions';
import { LayoutEditor } from '../LayoutEditor';
import { PathsEditor } from '../PathsEditor';
import { BudgetPanel } from '../BudgetPanel';
import { PartsPanel } from '../PartsPanel';
import { RenderPanel } from '../RenderPanel';
import { useInspectorView } from './InspectorView.hooks';

/** The Inspector: compare the project's versions, lay out its scene or draw its paths. */
export default function InspectorView() {
  const { project, view, loading, error, load, tab } = useInspectorView();

  if (project && tab === 'budget') {
    return (
      <InspectorShell>
        <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 py-[18px]">
          <InspectorTabs />
          <BudgetPanel />
        </main>
      </InspectorShell>
    );
  }
  if (project && tab === 'render') {
    return (
      <InspectorShell>
        <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 py-[18px]">
          <InspectorTabs />
          <RenderPanel />
        </main>
      </InspectorShell>
    );
  }
  if (project && tab === 'parts') {
    return (
      <InspectorShell>
        <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 py-[18px]">
          <InspectorTabs />
          <PartsPanel />
        </main>
      </InspectorShell>
    );
  }
  if (project && tab === 'layout') {
    return (
      <InspectorShell right={<LayoutBarActions />}>
        <LayoutEditor project={project.id} tabs={<InspectorTabs />} />
      </InspectorShell>
    );
  }
  if (project && tab === 'paths') {
    return (
      <InspectorShell>
        <PathsEditor project={project.id} tabs={<InspectorTabs />} />
      </InspectorShell>
    );
  }
  if (!project) {
    return (
      <InspectorShell>
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="cube-focus"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </InspectorShell>
    );
  }
  if (!view || view.project !== project.id) {
    return (
      <InspectorShell>
        <main className="flex flex-1 flex-col px-6 py-[18px]" aria-busy={loading}>
          <InspectorTabs />
          {error ? (
            <div className="grid flex-1 place-items-center">
              <EmptyState
                icon="warning-circle"
                title="The models could not be read"
                body={error}
                action={
                  <Button
                    icon="arrow-clockwise"
                    tooltip="Loads the Inspector again"
                    onClick={() => void load(project.id)}
                  >
                    Try again
                  </Button>
                }
              />
            </div>
          ) : (
            <div className="grid flex-1 place-items-center">
              <Loader />
            </div>
          )}
        </main>
      </InspectorShell>
    );
  }
  return <Inspector view={view} />;
}
