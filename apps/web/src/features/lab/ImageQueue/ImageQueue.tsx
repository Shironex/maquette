import { Button } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { queueFooter, queueSummary } from '@/state/lab';
import { QueueRow } from './QueueRow';
import { useImageQueue } from './ImageQueue.hooks';

export interface ImageQueueProps {
  view: LabView;
}

/** The right panel: every Codex job of this session, with its state. */
export default function ImageQueue({ view }: ImageQueueProps) {
  const { cancelAll, clearFinished, runNext, now } = useImageQueue();
  const jobs = view.jobs;
  const summary = queueSummary(jobs);
  const live = summary.running + summary.queued;
  const last = jobs[jobs.length - 1];
  const finished = jobs.length - live;
  const queued = jobs.filter(j => j.state === 'queued').length;
  const rows = jobs.map(job => (
    <li key={job.id}>
      <QueueRow job={job} projectDir={view.projectDir} now={now} />
    </li>
  ));

  return (
    <aside
      aria-label="Image queue"
      className="flex w-[248px] flex-none max-[1023px]:w-[200px] min-[1280px]:w-[296px] flex-col border-l border-line bg-panel px-4 pt-[18px] pb-3.5"
    >
      <div className="mb-1 flex items-baseline justify-between">
        <h2 className="text-panel-title">Image queue</h2>
        <span className="flex items-baseline gap-1.5">
          <span className="text-meta text-fg-mute tabular-nums">
            {jobs.length ? `${summary.done} of ${summary.total} done` : 'Empty'}
          </span>
          {queued > 0 ? (
            <Button
              size="sm"
              variant="ghost"
              tooltip="Moves this project's waiting images to the front of the line, ahead of other projects'"
              onClick={() => void runNext()}
            >
              Run next
            </Button>
          ) : null}
        </span>
      </div>
      <p className="mb-2 text-meta text-fg-mute">
        {last
          ? `${last.directionName} · ${last.backend === 'openai-images' ? 'OpenAI Images' : 'Codex CLI'}`
          : 'Images show here while they are made.'}
      </p>
      <ul
        aria-live="polite"
        className="-mx-1 flex min-h-0 flex-1 list-none flex-col overflow-y-auto px-1"
      >
        {rows}
      </ul>
      <div className="mt-auto flex items-center justify-between gap-2 border-t border-line pt-3">
        <span className="text-meta text-fg-mute">{queueFooter(summary, view.concurrency)}</span>
        {live > 0 ? (
          <Button
            size="sm"
            variant="ghost"
            tooltip={
              summary.running
                ? 'Stops the image being drawn and drops the ones waiting'
                : 'Drops every image waiting in the queue'
            }
            onClick={() => void cancelAll()}
          >
            {summary.running ? 'Cancel running' : 'Cancel all'}
          </Button>
        ) : finished > 0 ? (
          <Button
            size="sm"
            variant="ghost"
            tooltip="Removes the finished and failed rows from the queue"
            onClick={() => void clearFinished()}
          >
            Clear
          </Button>
        ) : null}
      </div>
    </aside>
  );
}
