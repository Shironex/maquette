import { Button, Icon, Select } from '@maquette/ui';
import type { Arrival } from '@/ipc/client';
import { formatAgo, formatBytes } from '@/lib/format';
import { useArrivalRow } from './ArrivalRow.hooks';

const NONE = '';

export interface ArrivalRowProps {
  arrival: Arrival;
  /** The waiting sends it can go to. */
  options: { value: string; label: string }[];
  now: number;
}

/** One download asking which asset it is: pick the send, then add it or skip it. */
export default function ArrivalRow({ arrival, options, now }: ArrivalRowProps) {
  const { choice, busy, choose, assign, dismiss } = useArrivalRow(arrival.id);
  const meta = [
    arrival.inner ? `holds ${arrival.inner}` : null,
    arrival.bytes !== null ? formatBytes(arrival.bytes) : null,
    arrival.atMs !== null ? formatAgo(arrival.atMs, now) : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <li className="flex flex-col gap-2 border-t border-line px-3 py-2.5 first:border-t-0">
      <div className="flex items-center gap-3">
        <span className="grid size-9 flex-none place-items-center rounded-control bg-studio text-fg-mute">
          <Icon name={arrival.name.toLowerCase().endsWith('.zip') ? 'package' : 'cube'} size={17} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="truncate font-mono text-[12.5px]" title={arrival.path}>
            {arrival.name}
          </div>
          <div className="truncate text-meta text-fg-mute">{meta}</div>
          <div
            className={
              arrival.state === 'failed'
                ? 'truncate text-meta text-danger'
                : 'truncate text-meta text-fg-dim'
            }
          >
            {arrival.reason}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-2 pl-12">
        <Select
          aria-label={`Asset for ${arrival.name}`}
          size="sm"
          options={[{ value: NONE, label: 'Pick the asset' }, ...options]}
          value={choice ?? NONE}
          onChange={value => choose(arrival.id, value)}
          fieldClassName="min-w-0 flex-1"
        />
        <Button size="sm" variant="ghost" onClick={() => void dismiss(arrival.id)}>
          Not a Maquette model
        </Button>
        <Button
          size="sm"
          variant={choice ? 'primary' : 'secondary'}
          disabled={!choice || busy}
          disabledReason="Pick the asset it belongs to"
          loading={busy}
          onClick={() => void assign(arrival.id)}
        >
          Add to inbox
        </Button>
      </div>
    </li>
  );
}
