import { EmptyState } from '@maquette/ui';
import { useAppStore } from '@/state/app-store';
import { useLabStore } from '@/state/lab-store';
import { ProjectTopBar } from '@/features/shell/ProjectTopBar';
import { Composer } from './Composer';
import { ConceptGrid } from './ConceptGrid';
import { ImageQueue } from './ImageQueue';
import { Lightbox } from './Lightbox';

/** The Concept lab: the brief on the left, the results in the middle, the Codex queue on the right. */
export function LabView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const view = useLabStore(s => s.view);
  const ready = view && project && view.project === project.id;
  return (
    <>
      <ProjectTopBar />
      {ready ? (
        <main className="flex min-h-0 flex-1">
          <Composer view={view} />
          <ConceptGrid view={view} />
          <ImageQueue view={view} />
          <Lightbox view={view} />
        </main>
      ) : (
        <main className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyState
            icon="flask"
            title={project ? 'Loading the concepts' : 'No project open'}
            body={
              project ? undefined : 'Create or open a project in Settings to generate concepts.'
            }
          />
        </main>
      )}
    </>
  );
}
