import { ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { ConceptJob, Direction } from '@/ipc/client';
import { formatClock } from '@/lib/format';
import { aspectCss, formatElapsed, tileLabel } from '@/state/lab';
import { useJobTile } from './JobTile.hooks';

export interface JobTileProps {
  direction: Direction;
  job: ConceptJob;
}

/** A concept still in the queue, generating or waiting out a rate or usage limit, with its time. */
export default function JobTile({ direction, job }: JobTileProps) {
  const now = useJobTile();
  const running = job.state === 'running';
  const limited = job.state === 'rate-limited';
  const approval = job.state === 'approval';
  const elapsed =
    running && job.startedAtMs !== null
      ? formatElapsed(now - job.startedAtMs)
      : limited && job.retryAtMs !== null
        ? `${job.usageLimit ? 'until about' : 'retries at'} ${formatClock(job.retryAtMs)}`
        : approval
          ? 'an agent asked'
          : 'in line';
  return (
    <figure className="m-0 flex flex-col gap-[7px]">
      <div
        className="flex flex-col items-center justify-center gap-2.5 rounded-[10px] border border-line bg-raised"
        style={{ aspectRatio: aspectCss(direction.aspect) }}
      >
        <StatusChip
          status={running ? 'running' : 'queued'}
          label={
            running
              ? 'generating'
              : limited
                ? job.usageLimit
                  ? 'usage limit'
                  : 'rate limited'
                : approval
                  ? 'needs approval'
                  : 'waiting'
          }
        />
        {running ? <ProgressBar className="w-[140px]" /> : null}
      </div>
      <figcaption className="flex items-center justify-between">
        <span className="min-w-0 truncate text-fg-mute">{tileLabel(job)}</span>
        <span
          className={cn(
            'text-meta tabular-nums',
            limited || approval ? 'text-amber' : 'text-fg-mute'
          )}
        >
          {elapsed}
        </span>
      </figcaption>
    </figure>
  );
}
