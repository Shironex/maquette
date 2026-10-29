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
 * A download that looks like a waiting send's model (by its picture or its
 * name): its front render next to the send's front ref, how alike they
 * are and why, the other waiting sends ranked after it, and attach it,
 * pick another asset or leave it.
 */
export default function LookCard({ arrival, now }: LookCardProps) {
  const { ref, render, fit, others, here, busy, attach, pickAnother, notThis } =
    useLookCard(arrival);
  const asset = arrival.assetName ?? 'its asset';
  const otherButtons = others.map(o => (
    <Button
      key={o.job}
      size="sm"
      variant="ghost"
      disabled={busy || !o.here}
      disabledReason="Open its project to attach it"
      tooltip="Sends it to the Inbox for this asset instead"
      onClick={o.attach}
    >
      {o.label}
    </Button>
  ));
  return (
    <li className="flex flex-col gap-2.5 border-t border-line px-3 py-3 first:border-t-0">
      <div className="flex items-center gap-3">
        <div className="flex flex-none items-center gap-1.5">
          <figure className="m-0 flex flex-col items-center gap-1">
            {render ? (
              <span className="grid size-[72px] place-items-center overflow-hidden rounded-control bg-studio">
                <img
                  src={render}
                  alt={`${arrival.name} from the front`}
                  className="size-full object-contain"
                />
              </span>
            ) : (
              <DownloadThumb arrival={arrival} className="size-[72px]" />
            )}
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
            <figcaption className="text-[11px] text-fg-mute">Front ref</figcaption>
          </figure>
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2">
            <span className="truncate font-semibold">Is this {asset}?</span>
            {fit ? (
              <span className="flex-none text-meta text-clay-text tabular-nums">{fit}</span>
            ) : null}
          </div>
          <div className="truncate font-mono text-[12.5px]" title={arrival.path}>
            {arrival.name}
          </div>
          <div className="truncate text-meta text-fg-mute">{arrivalMeta(arrival, now)}</div>
          <div className="truncate text-meta text-fg-dim">{arrival.reason}</div>
        </div>
      </div>
      {otherButtons.length > 0 ? (
        <div className="flex items-start gap-2">
          <span className="flex-none py-1 text-meta text-fg-mute">Also waiting:</span>
          <div className="flex min-w-0 flex-1 flex-wrap gap-1.5">{otherButtons}</div>
        </div>
      ) : null}
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
