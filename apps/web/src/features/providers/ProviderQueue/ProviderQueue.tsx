/**
 * The 3D queue: every send of the open project with its provider,
 * progress, credits and time, newest first. Cancel where the provider
 * allows it; a By hand send opens its card.
 */
import { Button, Icon, IconButton } from '@maquette/ui';
import { CreditsButton } from '../CreditsButton';
import { useDismissOutside, useQueueJobs } from './ProviderQueue.hooks';
import { QueueRow } from './QueueRow';
import { KEYS } from '@/lib/keys';

export interface ProviderQueueProps {
  onClose: () => void;
}

export default function ProviderQueue({ onClose }: ProviderQueueProps) {
  const { sorted, finished, clear, now, project } = useQueueJobs();
  const ref = useDismissOutside(onClose);
  const rows = sorted.map(job => <QueueRow key={job.id} job={job} now={now} />);

  return (
    <section
      ref={ref}
      aria-label="3D queue"
      className="absolute top-10 right-0 z-30 flex max-h-[min(640px,calc(100vh-160px))] w-[440px] flex-col rounded-card border border-line-strong bg-panel shadow-window"
    >
      <header className="flex items-center gap-2 border-b border-line px-3.5 py-2.5">
        <Icon name="cube" className="text-fg-mute" />
        <h2 className="flex-1 font-semibold">3D queue</h2>
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
        <IconButton
          icon="x"
          label="Close the queue"
          shortcut={KEYS.escape}
          size="sm"
          onClick={onClose}
        />
      </header>
      {sorted.length === 0 ? (
        <p className="px-3.5 py-6 text-center text-meta text-fg-mute">
          Nothing sent yet. Send an asset from its card or from a picked concept in the Lab.
        </p>
      ) : (
        <ul className="m-0 min-h-0 flex-1 list-none overflow-y-auto px-3.5 py-1">{rows}</ul>
      )}
    </section>
  );
}
