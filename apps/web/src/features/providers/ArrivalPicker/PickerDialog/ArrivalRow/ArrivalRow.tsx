import { Button } from '@maquette/ui';
import type { Arrival } from '@/ipc/client';
import { arrivalMeta } from '@/state/downloads';
import { DownloadThumb } from '../DownloadThumb';
import { useArrivalRow } from './ArrivalRow.hooks';

export interface ArrivalRowProps {
  arrival: Arrival;
  now: number;
}

/** One download asking which asset it is: attach it to one, or skip it. */
export default function ArrivalRow({ arrival, now }: ArrivalRowProps) {
  const { busy, choose, dismiss } = useArrivalRow(arrival.id);
  return (
    <li className="flex items-center gap-3 border-t border-line px-3 py-2.5 first:border-t-0">
      <DownloadThumb arrival={arrival} className="size-11" />
      <div className="min-w-0 flex-1">
        <div className="truncate font-mono text-[12.5px]" title={arrival.path}>
          {arrival.name}
        </div>
        <div className="truncate text-meta text-fg-mute">{arrivalMeta(arrival, now)}</div>
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
      <Button
        size="sm"
        variant="ghost"
        tooltip="Hides this file; nothing is added"
        onClick={dismiss}
      >
        Not a Maquette model
      </Button>
      <Button
        size="sm"
        variant="primary"
        loading={busy}
        disabled={busy}
        tooltip="Pick the asset this model is for"
        onClick={choose}
      >
        Attach to an asset
      </Button>
    </li>
  );
}
