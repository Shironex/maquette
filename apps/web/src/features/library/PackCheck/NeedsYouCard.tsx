import { Button, Icon } from '@maquette/ui';
import type { PackDecision, PackItem } from '@/ipc/client';
import { formatCount } from '@/lib/format';

export interface NeedsYouCardProps {
  item: PackItem;
  busy: boolean;
  onDecide: (decision: PackDecision) => void;
}

/**
 * A model the check flagged, with the answers it takes: a failed one is
 * exported again; one that lost thin parts is raised to the budget that
 * keeps them or kept as it is; any other warning is kept or exported again.
 */
export default function NeedsYouCard({ item, busy, onDecide }: NeedsYouCardProps) {
  const key = item.model.key;
  const failed = item.state === 'fail';
  let actions;
  if (failed) {
    actions = (
      <Button
        size="sm"
        variant="primary"
        icon="arrow-clockwise"
        disabled={busy}
        tooltip="Runs this model in Blender again, after you fixed its source"
        onClick={() => onDecide({ choice: 'again', key })}
      >
        Export again
      </Button>
    );
  } else if (item.suggest !== null && item.budget !== null) {
    const suggest = item.suggest;
    actions = (
      <>
        <Button
          size="sm"
          variant="primary"
          disabled={busy}
          tooltip="Decimates it again to the higher budget, then checks it"
          onClick={() => onDecide({ choice: 'raise', key, triangles: suggest })}
        >
          Raise to {formatCount(suggest)}
        </Button>
        <Button
          size="sm"
          disabled={busy}
          tooltip="Lets it into the pack as the check found it"
          onClick={() => onDecide({ choice: 'keep', key })}
        >
          Keep {formatCount(item.budget)}
        </Button>
      </>
    );
  } else {
    actions = (
      <>
        <Button
          size="sm"
          variant="primary"
          disabled={busy}
          onClick={() => onDecide({ choice: 'keep', key })}
        >
          Keep it
        </Button>
        <Button size="sm" disabled={busy} onClick={() => onDecide({ choice: 'again', key })}>
          Export again
        </Button>
      </>
    );
  }
  return (
    <article
      aria-label={`${item.model.name} needs you`}
      className={`flex flex-col gap-2 rounded-card border p-3 ${
        failed ? 'border-danger/50 bg-danger-soft' : 'border-clay-line bg-clay-soft'
      }`}
    >
      <div className="flex items-center gap-2">
        <Icon
          name={failed ? 'x-circle' : 'warning'}
          size={16}
          className={failed ? 'text-danger' : 'text-clay-text'}
        />
        <b className="min-w-0 flex-1 truncate">{item.model.name}</b>
        <span className="font-mono text-meta text-fg-mute">{failed ? 'failed' : 'to look at'}</span>
      </div>
      <p className="text-meta text-fg-dim">{item.note}</p>
      <div className="flex flex-wrap gap-2">{actions}</div>
    </article>
  );
}
