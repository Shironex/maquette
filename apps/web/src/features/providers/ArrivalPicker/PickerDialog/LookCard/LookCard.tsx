import { Button, Icon } from '@maquette/ui';
import type { Arrival } from '@/ipc/client';
import { arrivalMeta } from '@/state/downloads';
import { DownloadThumb } from '../DownloadThumb';
import { useLookCard } from './LookCard.hooks';

export interface LookCardProps {
  arrival: Arrival;
  now: number;
}

/**
 * A download whose name is only close to a waiting send's: the model next
 * to the ref it was sent with, why it matched, and attach it, pick another
 * asset or leave it.
 */
export default function LookCard({ arrival, now }: LookCardProps) {
  const { ref, here, busy, attach, pickAnother, notThis } = useLookCard(arrival);
  const asset = arrival.assetName ?? 'its asset';
  return (
    <li className="flex flex-col gap-2.5 border-t border-line px-3 py-3 first:border-t-0">
      <div className="flex items-center gap-3">
        <div className="flex flex-none items-center gap-1.5">
          <figure className="m-0 flex flex-col items-center gap-1">
            <DownloadThumb arrival={arrival} className="size-[72px]" />
            <figcaption className="text-[11px] text-fg-mute">Download</figcaption>
          </figure>
          <Icon name="arrow-right" size={14} className="mb-4 text-fg-mute" />
          <figure className="m-0 flex flex-col items-center gap-1">
            <span className="grid size-[72px] place-items-center overflow-hidden rounded-control bg-studio text-fg-mute">
              {ref ? (
                <img
                  src={ref}
                  alt={`The ref sent for ${asset}`}
                  className="size-full object-cover"
                />
              ) : (
                <Icon name="image" size={17} />
              )}
            </span>
            <figcaption className="text-[11px] text-fg-mute">Sent ref</figcaption>
          </figure>
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-semibold">Is this {asset}?</div>
          <div className="truncate font-mono text-[12.5px]" title={arrival.path}>
            {arrival.name}
          </div>
          <div className="truncate text-meta text-fg-mute">{arrivalMeta(arrival, now)}</div>
          <div className="truncate text-meta text-fg-dim">{arrival.reason}</div>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">
        <Button
          size="sm"
          variant="ghost"
          tooltip="Leaves the file alone and does not ask about it again"
          onClick={notThis}
        >
          Not this one
        </Button>
        <Button
          size="sm"
          variant="secondary"
          tooltip="Choose the asset it is for"
          onClick={pickAnother}
        >
          Pick another asset
        </Button>
        <Button
          size="sm"
          variant="primary"
          loading={busy}
          disabled={busy || !here || !arrival.asset}
          disabledReason="Open its project to attach it"
          tooltip={`Sends it to the Inbox for ${asset}`}
          onClick={attach}
        >
          Attach to {asset}
        </Button>
      </div>
    </li>
  );
}
