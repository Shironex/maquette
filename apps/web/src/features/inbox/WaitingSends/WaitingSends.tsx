import { Button } from '@maquette/ui';
import { formatAgo } from '@/lib/format';
import { SERVICE_LABEL } from '@/state/providers';
import { useWaitingSends } from './WaitingSends.hooks';

/**
 * The By hand sends still waiting for a model, each with Open handoff and
 * Stop waiting, so a round can be closed from the empty Inbox.
 */
export default function WaitingSends() {
  const { now, waiting, openHandoff, cancel } = useWaitingSends();
  if (waiting.length === 0) return null;
  const rows = waiting.map(j => (
    <li
      key={j.id}
      className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-card border border-line bg-raised px-3 py-2"
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate font-semibold">{j.assetName} still waits</span>
        <span className="block truncate text-meta text-fg-mute">
          {SERVICE_LABEL[j.service]} by hand
          {j.createdAtMs != null ? ` · sent ${formatAgo(j.createdAtMs, now)}` : ''}
        </span>
      </span>
      <span className="flex gap-2">
        <Button
          size="sm"
          variant="ghost"
          icon="folder-open"
          tooltip="Opens the handoff folder with its refs and settings card"
          onClick={() => void openHandoff(j.id)}
        >
          Open handoff
        </Button>
        <Button
          size="sm"
          tooltip="Stops waiting for this model; nothing was spent"
          onClick={() => void cancel(j.id)}
        >
          Stop waiting
        </Button>
      </span>
    </li>
  ));
  return (
    <section aria-labelledby="waiting-title" className="flex flex-col gap-2">
      <h3
        id="waiting-title"
        className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
      >
        Still waiting · {waiting.length} {waiting.length === 1 ? 'send' : 'sends'}
      </h3>
      <ul className="flex list-none flex-col gap-1.5">{rows}</ul>
    </section>
  );
}
