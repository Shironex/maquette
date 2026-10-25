import { Button, Icon, ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { isActive, jobChip, jobMeta, jobTitle, percent } from '@/state/providers';
import { KIND_ICON } from '@/state/thumbs';
import { CANCEL_LABEL } from './cancelLabel';
import { useQueueRow } from './QueueRow.hooks';

export interface QueueRowProps {
  job: ProviderJob;
  now: number;
}

export default function QueueRow({ job, now }: QueueRowProps) {
  const { asset, thumb, cancel, showHandoff, busy } = useQueueRow(job);
  const chip = jobChip(job);
  const cancelLabel = isActive(job.state) ? CANCEL_LABEL[job.cancel] : null;
  const failed = job.state === 'failed';
  const showMessage = job.message && job.state !== 'running' && job.message !== chip.label;
  return (
    <li className="flex gap-3 border-t border-line py-3 first:border-t-0">
      <span className="grid h-10 w-14 flex-none place-items-center overflow-hidden rounded-control bg-studio">
        {thumb ? (
          <img src={thumb} alt="" className="size-full object-contain" />
        ) : (
          <Icon name={asset ? KIND_ICON[asset.kind] : 'cube'} className="text-fg-mute" />
        )}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="min-w-0 flex-1 truncate font-semibold">{job.assetName}</span>
          <StatusChip status={chip.status} label={chip.label} icon={chip.icon} />
        </div>
        <div className="truncate text-meta text-fg-dim">{jobTitle(job)}</div>
        <div className="truncate font-mono text-[11.5px] text-fg-mute tabular-nums">
          {jobMeta(job, now)}
        </div>
        {job.state === 'running' ? <ProgressBar value={percent(job)} className="mt-0.5" /> : null}
        {job.provider === 'tripo' && job.state === 'running' ? (
          <p className="text-meta text-fg-mute">
            Tripo cannot cancel a sent task, so this one finishes and lands in the Inbox.
          </p>
        ) : null}
        {showMessage ? (
          <p
            className={cn(
              'text-meta',
              failed ? 'rounded-[7px] bg-danger-soft px-2 py-1.5 text-danger' : 'text-fg-mute'
            )}
          >
            {job.message}
          </p>
        ) : null}
        {cancelLabel || job.provider === 'by-hand' ? (
          <div className="mt-0.5 flex gap-1.5">
            {job.provider === 'by-hand' && job.card ? (
              <Button size="sm" icon="file-text" onClick={() => showHandoff(job)}>
                Settings card
              </Button>
            ) : null}
            {cancelLabel ? (
              <Button
                size="sm"
                variant="ghost"
                loading={busy}
                disabled={busy}
                onClick={() => void cancel(job.id)}
              >
                {cancelLabel}
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>
    </li>
  );
}
