import { Button, EmptyState } from '@maquette/ui';
import { Inspector } from '../Inspector';
import { InspectorShell } from '../InspectorShell';
import { InspectorTabs } from '../InspectorTabs';
import { LayoutBarActions } from '../LayoutBarActions';
import { LayoutEditor } from '../LayoutEditor';
import { useInspectorView } from './InspectorView.hooks';

/** The Inspector: compare the project's versions, or lay out its scene. */
export default function InspectorView() {
  const { project, view, loading, error, load, tab } = useInspectorView();

  if (project && tab === 'layout') {
    return (
      <InspectorShell right={<LayoutBarActions />}>
        <LayoutEditor project={project.id} tabs={<InspectorTabs />} />
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
                  <Button icon="arrow-clockwise" onClick={() => void load(project.id)}>
                    Try again
                  </Button>
                }
              />
            </div>
          ) : null}
        </main>
      </InspectorShell>
    );
  }
  return <Inspector view={view} />;
}
