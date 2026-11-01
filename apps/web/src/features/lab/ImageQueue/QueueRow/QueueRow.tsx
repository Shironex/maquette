import { Icon, IconButton, StatusChip, Tooltip, cn } from '@maquette/ui';
import type { ConceptJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { canRetry, inProject, jobMeta, jobStatus, tileLabel } from '@/state/lab';
import { useQueueRow } from './QueueRow.hooks';

export interface QueueRowProps {
  job: ConceptJob;
  projectDir: string;
  now: number;
  /** How long the job finished last took, said beside one still going. */
  lastMs?: number | null;
}

/**
 * One Codex job: its thumbnail once done, its state, Run next while it
 * waits in the queue, Cancel while it is live and Retry once it failed or
 * waits out a rate limit.
 */
export default function QueueRow({ job, projectDir, now, lastMs = null }: QueueRowProps) {
  const { cancel, retry, runNext, openLightbox } = useQueueRow();
  const path = inProject(projectDir, job.file);
  const thumb = job.state === 'done' ? fileUrl(path) : null;
  const limited = job.state === 'rate-limited';
  const approval = job.state === 'approval';
  const live = job.state === 'running' || job.state === 'queued' || limited || approval;
  const failed = job.state === 'failed' || job.state === 'cancelled';
  // A waiting, rate limited, held or refused row says it in its own line, and
  // a waiting one needs the room for Run next and Cancel.
  const queued = job.state === 'queued';
  const chip = !(job.state === 'running' || queued || limited || approval || job.refused);
  const label = tileLabel(job);
  return (
    <div className="group flex items-center gap-2.5 border-t border-line py-2.5 first:border-t-0">
      {thumb ? (
        <Tooltip label="Opens it large">
          <button
            type="button"
            aria-label={`Open ${label}`}
            onClick={() => openLightbox(path)}
            className="h-9 w-14 flex-none cursor-zoom-in overflow-hidden rounded-control"
          >
            <img src={thumb} alt="" className="size-full object-cover" />
          </button>
        </Tooltip>
      ) : (
        <span
          className={cn(
            'h-9 w-14 flex-none rounded-control',
            failed ? 'bg-danger-soft' : 'bg-hover',
            'grid place-items-center text-clay-text'
          )}
        >
          {job.state === 'running' ? <Icon name="circle-notch" size={14} spin /> : null}
        </span>
      )}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{label}</span>
        <span
          className={cn(
            'text-[11.5px] tabular-nums',
            failed
              ? 'line-clamp-2 text-danger'
              : limited || approval
                ? 'line-clamp-2 text-amber'
                : 'text-fg-mute'
          )}
          title={
            job.refused
              ? (job.message ?? undefined)
              : failed
                ? job.logTail.slice(-6).join('\n') || undefined
                : undefined
          }
        >
          {jobMeta(job, now, lastMs)}
        </span>
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
      {queued ? (
        <IconButton
          icon="arrow-up"
          label={`Run ${label} next`}
          hint="Moves it to the front of the line, ahead of other projects' images"
          size="sm"
          variant="ghost"
          onClick={() => void runNext([job.id])}
          className="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
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
      {!chip ? null : (
        <StatusChip
          status={jobStatus(job)}
          label={job.state === 'cancelled' ? 'cancelled' : undefined}
        />
      )}
    </div>
  );
}
