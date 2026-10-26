import { IconButton, ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { ConceptJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { canRetry, formatElapsed, inProject, jobMeta, jobStatus, tileLabel } from '@/state/lab';
import { useQueueRow } from './QueueRow.hooks';

export interface QueueRowProps {
  job: ConceptJob;
  projectDir: string;
  now: number;
}

/**
 * One Codex job: its thumbnail once done, its state, Cancel while it is
 * live and Retry once it failed or waits out a rate limit.
 */
export default function QueueRow({ job, projectDir, now }: QueueRowProps) {
  const { cancel, retry, openLightbox } = useQueueRow();
  const path = inProject(projectDir, job.file);
  const thumb = job.state === 'done' ? fileUrl(path) : null;
  const limited = job.state === 'rate-limited';
  const live = job.state === 'running' || job.state === 'queued' || limited;
  const failed = job.state === 'failed' || job.state === 'cancelled';
  const label = tileLabel(job);
  const elapsed =
    job.state === 'running' && job.startedAtMs !== null
      ? formatElapsed(now - job.startedAtMs)
      : null;
  return (
    <div className="group flex items-center gap-2.5 border-t border-line py-2.5 first:border-t-0">
      {thumb ? (
        <button
          type="button"
          aria-label={`Open ${label}`}
          onClick={() => openLightbox(path)}
          className="h-9 w-14 flex-none cursor-zoom-in overflow-hidden rounded-control"
        >
          <img src={thumb} alt="" className="size-full object-cover" />
        </button>
      ) : (
        <span
          className={cn(
            'h-9 w-14 flex-none rounded-control',
            failed ? 'bg-danger-soft' : 'bg-hover'
          )}
        />
      )}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-center justify-between gap-2">
          <span className="truncate font-medium">{label}</span>
          {elapsed !== null ? (
            <span className="text-meta text-amber tabular-nums">{elapsed}</span>
          ) : null}
        </div>
        {job.state === 'running' ? (
          <ProgressBar className="min-w-0" label={undefined} />
        ) : (
          <span
            className={cn(
              'text-[11.5px] tabular-nums',
              failed ? 'line-clamp-2 text-danger' : limited ? 'text-amber' : 'text-fg-mute'
            )}
            title={failed ? job.logTail.slice(-6).join('\n') || undefined : undefined}
          >
            {jobMeta(job, now)}
          </span>
        )}
      </div>
      {canRetry(job) ? (
        <IconButton
          icon="arrow-clockwise"
          label={limited ? `Retry ${label} now` : `Retry ${label}`}
          size="sm"
          variant="ghost"
          onClick={() => void retry(job.id)}
        />
      ) : null}
      {live ? (
        <IconButton
          icon="x"
          label={`Cancel ${label}`}
          size="sm"
          variant="ghost"
          onClick={() => void cancel(job.id)}
          className="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
        />
      ) : null}
      {/* A rate limited row says it in its own line, which needs the room. */}
      {job.state === 'running' || limited ? null : (
        <StatusChip
          status={jobStatus(job)}
          label={
            job.state === 'cancelled' ? 'cancelled' : job.state === 'queued' ? 'waiting' : undefined
          }
        />
      )}
    </div>
  );
}
