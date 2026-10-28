import { EmptyState } from '@maquette/ui';
import { ProjectTopBar } from '@/features/common/ProjectTopBar';
import { Composer } from '../Composer';
import { ConceptGrid } from '../ConceptGrid';
import { ImageQueue } from '../ImageQueue';
import { Lightbox } from '../Lightbox';
import { UsageLimitBanner } from '../UsageLimitBanner';
import { useLabView } from './LabView.hooks';

/**
 * The Concept lab: the brief on the left, the results in the middle (under
 * the usage limit banner while one holds), the Codex queue on the right.
 */
export default function LabView() {
  const { project, view } = useLabView();
  const ready = view && project && view.project === project.id;
  return (
    <>
      <ProjectTopBar />
      {ready ? (
        <main className="flex min-h-0 flex-1">
          <Composer view={view} />
          <div className="flex min-h-0 min-w-0 flex-1 flex-col">
            <UsageLimitBanner view={view} />
            <ConceptGrid view={view} />
          </div>
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
