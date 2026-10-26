import { Button, EmptyState } from '@maquette/ui';
import { useTimelineEmpty } from './TimelineEmpty.hooks';

const STEPS = [
  {
    title: 'Add camera stops in the Inspector',
    detail: 'Each stop is one section of the page. The timeline puts them in a row.',
  },
  {
    title: 'Give a prop states in the Motion view',
    detail: 'Then set where on the page it changes, or let a model spread apart.',
  },
];

/** The timeline's columns are the camera stops, so without them there is nothing to lay out. */
export default function TimelineEmpty() {
  const { name, openInspector } = useTimelineEmpty();
  return (
    <section aria-label="No camera stops yet" className="grid flex-1 place-items-center">
      <EmptyState
        icon="film-strip"
        title={`${name} has no camera stops yet`}
        body="The timeline runs along the page from one camera stop to the next."
        steps={STEPS}
        action={
          <Button variant="primary" icon="cube-focus" onClick={openInspector}>
            Open the Inspector
          </Button>
        }
      />
    </section>
  );
}
