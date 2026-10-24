/**
 * The 3D queue: every send of the open project with its provider,
 * progress, credits and time, newest first. Cancel where the provider
 * allows it; a By hand send opens its card.
 */
import { useEffect, useRef } from 'react';
import { Button, Icon, IconButton, ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { ProviderJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/app-store';
import { projectFile } from '@/state/board';
import { isActive, jobChip, jobMeta, jobTitle, percent, sortJobs } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';
import { KIND_ICON } from '@/state/thumbs';

const CANCEL_LABEL: Record<ProviderJob['cancel'], string | null> = {
  free: 'Cancel',
  provider: 'Cancel at Meshy',
  'stop-waiting': 'Stop waiting',
  none: null,
};

export function ProviderQueue({ onClose }: { onClose: () => void }) {
  const jobs = useProvidersStore(s => s.jobs);
  const clear = useProvidersStore(s => s.clearFinished);
  const ref = useRef<HTMLDivElement>(null);
  const now = useNow(1_000);
  const sorted = sortJobs(jobs);
  const finished = jobs.some(j => !isActive(j.state) && j.state !== 'inbox');

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const onDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement;
      if (ref.current?.contains(target) || target.closest('[data-queue-toggle]')) return;
      onClose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onDown);
    };
  }, [onClose]);

  return (
    <section
      ref={ref}
      aria-label="3D queue"
      className="absolute top-10 right-0 z-30 flex max-h-[min(640px,calc(100vh-160px))] w-[440px] flex-col rounded-card border border-line-strong bg-panel shadow-window"
    >
      <header className="flex items-center gap-2 border-b border-line px-3.5 py-2.5">
        <Icon name="cube" className="text-fg-mute" />
        <h2 className="flex-1 font-semibold">3D queue</h2>
        <Button size="sm" variant="ghost" disabled={!finished} onClick={() => void clear()}>
          Clear finished
        </Button>
        <IconButton icon="x" label="Close the queue" size="sm" onClick={onClose} />
      </header>
      {sorted.length === 0 ? (
        <p className="px-3.5 py-6 text-center text-meta text-fg-mute">
          Nothing sent yet. Send an asset from its card or from a picked concept in the Lab.
        </p>
      ) : (
        <ul className="m-0 min-h-0 flex-1 list-none overflow-y-auto px-3.5 py-1">
          {sorted.map(job => (
            <QueueRow key={job.id} job={job} now={now} />
          ))}
        </ul>
      )}
    </section>
  );
}

function QueueRow({ job, now }: { job: ProviderJob; now: number }) {
  const workspace = useAppStore(s => s.workspace);
  const asset = workspace?.project?.assets?.find(a => a.id === job.asset);
  const cancel = useProvidersStore(s => s.cancel);
  const showHandoff = useProvidersStore(s => s.showHandoff);
  const busy = useProvidersStore(s => s.busy.includes(job.id));
  const thumbPath = asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
  const thumb = fileUrl(thumbPath);
  const chip = jobChip(job);
  const cancelLabel = isActive(job.state) ? CANCEL_LABEL[job.cancel] : null;
  const failed = job.state === 'failed';
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
        {job.message && job.state !== 'running' && job.message !== chip.label ? (
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
