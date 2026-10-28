import { EmptyState } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { DirectionResults } from './DirectionResults';
import { useConceptGrid } from './ConceptGrid.hooks';

export interface ConceptGridProps {
  view: LabView;
}

/** The middle column: the selected direction's images and the ones still generating. */
export default function ConceptGrid({ view }: ConceptGridProps) {
  const direction = useConceptGrid(view);
  if (!direction) {
    return (
      <section className="flex min-h-0 min-w-0 flex-1 items-center justify-center p-8">
        <EmptyState
          icon="flask"
          title="No concepts yet"
          body="Name a direction, describe it and press Generate. Codex makes each image in about a minute, a few at a time."
          steps={[
            {
              title: 'Write the brief',
              detail: 'A direction name, the prompt and any references.',
            },
            { title: 'Generate', detail: 'Images land in concepts/ inside the project.' },
            { title: 'Star and pick', detail: 'Send the picks to Refs to start the asset.' },
          ]}
        />
      </section>
    );
  }
  return <DirectionResults view={view} direction={direction} />;
}
