import { Button, IconButton, ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { ConceptJob, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useNow } from '@/lib/useNow';
import {
  tileLabel,
  formatElapsed,
  inProject,
  jobMeta,
  jobStatus,
  queueFooter,
  queueSummary,
} from '@/state/lab';
import { useLabStore } from '@/state/lab-store';

/** The right panel: every Codex job of this session, with its state. */
export function ImageQueue({ view }: { view: LabView }) {
  const cancelAll = useLabStore(s => s.cancelAll);
  const clearFinished = useLabStore(s => s.clearFinished);
  const now = useNow(1000);
  const jobs = view.jobs;
  const summary = queueSummary(jobs);
  const live = summary.running + summary.queued;
  const last = jobs[jobs.length - 1];
  const finished = jobs.length - live;

  return (
    <aside
      aria-label="Image queue"
      className="flex w-[296px] flex-none flex-col border-l border-line bg-panel px-4 pt-[18px] pb-3.5"
    >
      <div className="mb-1 flex items-baseline justify-between">
        <h2 className="text-panel-title">Image queue</h2>
        <span className="text-meta text-fg-mute tabular-nums">
          {jobs.length ? `${summary.done} of ${summary.total} done` : 'Empty'}
        </span>
      </div>
      <p className="mb-2 text-meta text-fg-mute">
        {last ? `${last.directionName} · Codex CLI` : 'Images show here while Codex works on them.'}
      </p>
      <ul
        aria-live="polite"
        className="-mx-1 flex min-h-0 flex-1 list-none flex-col overflow-y-auto px-1"
      >
        {jobs.map(job => (
          <li key={job.id}>
            <QueueRow job={job} projectDir={view.projectDir} now={now} />
          </li>
        ))}
      </ul>
      <div className="mt-auto flex items-center justify-between gap-2 border-t border-line pt-3">
        <span className="text-meta text-fg-mute">{queueFooter(summary, view.concurrency)}</span>
        {live > 0 ? (
          <Button size="sm" variant="ghost" onClick={() => void cancelAll()}>
            {summary.running ? 'Cancel running' : 'Cancel all'}
          </Button>
        ) : finished > 0 ? (
          <Button size="sm" variant="ghost" onClick={() => void clearFinished()}>
            Clear
          </Button>
        ) : null}
      </div>
    </aside>
  );
}

function QueueRow({ job, projectDir, now }: { job: ConceptJob; projectDir: string; now: number }) {
  const cancel = useLabStore(s => s.cancel);
  const openLightbox = useLabStore(s => s.openLightbox);
  const path = inProject(projectDir, job.file);
  const thumb = job.state === 'done' ? fileUrl(path) : null;
  const live = job.state === 'running' || job.state === 'queued';
  const failed = job.state === 'failed' || job.state === 'cancelled';
  const label = tileLabel(job);
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
          {job.state === 'running' && job.startedAtMs !== null ? (
            <span className="text-meta text-amber tabular-nums">
              {formatElapsed(now - job.startedAtMs)}
            </span>
          ) : null}
        </div>
        {job.state === 'running' ? (
          <ProgressBar className="min-w-0" label={undefined} />
        ) : (
          <span
            className={cn(
              'text-[11.5px] tabular-nums',
              failed ? 'line-clamp-2 text-danger' : 'text-fg-mute'
            )}
            title={failed ? job.logTail.slice(-6).join('\n') || undefined : undefined}
          >
            {jobMeta(job, now)}
          </span>
        )}
      </div>
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
      {job.state === 'running' ? null : (
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
