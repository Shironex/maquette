/**
 * The 3D queue: every send of the open project with its provider,
 * progress, credits and time, newest first. Cancel where the provider
 * allows it; a By hand send opens its card.
 */
import { Button } from '@maquette/ui';
import { CreditsButton } from '../CreditsButton';
import { useQueueJobs } from './ProviderQueue.hooks';
import { QueueRow } from './QueueRow';

export interface ProviderQueueProps {
  /** Closes the panel it sits in, before the Credits sheet opens. */
  onClose: () => void;
}

export default function ProviderQueue({ onClose }: ProviderQueueProps) {
  const { sorted, finished, clear, now, project } = useQueueJobs();
  const rows = sorted.map(job => <QueueRow key={job.id} job={job} now={now} />);

  return (
    <section aria-label="3D queue" className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-line px-5 py-2.5">
        <CreditsButton project={project} onOpen={onClose} />
        <Button
          size="sm"
          variant="ghost"
          disabled={!finished}
          disabledReason="Nothing has finished yet"
          tooltip="Removes the finished and failed sends from this list"
          onClick={() => void clear()}
        >
          Clear finished
        </Button>
      </div>
      {sorted.length === 0 ? (
        <p className="px-5 py-6 text-center text-meta text-fg-mute">
          Nothing sent yet. Send an asset from its card or from a picked concept in the Lab.
        </p>
      ) : (
        <ul className="m-0 min-h-0 flex-1 list-none overflow-y-auto px-5 py-1">{rows}</ul>
      )}
    </section>
  );
}
