import { cn } from '../../lib/cn';
import { ProgressBar } from '../status/ProgressBar';
import { StatusChip, type Status } from '../status/StatusChip';

export interface JobRowProps {
  label: string;
  meta?: string;
  thumb?: string;
  state?: Status;
  progress?: number;
  className?: string;
}

export function JobRow({ label, meta, thumb, state = 'queued', progress, className }: JobRowProps) {
  const running = state === 'running' && progress !== undefined;
  return (
    <div
      className={cn(
        'flex items-center gap-2.5 border-t border-line py-2.5 first:border-t-0',
        className
      )}
    >
      {thumb ? (
        <img src={thumb} alt="" className="h-9 w-14 flex-none rounded-control object-cover" />
      ) : (
        <span className="h-9 w-14 flex-none rounded-control bg-hover" />
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{label}</div>
        {meta ? <div className="text-meta text-fg-mute tabular-nums">{meta}</div> : null}
        {running ? <ProgressBar value={progress} className="mt-1.5 min-w-0" /> : null}
      </div>
      <StatusChip status={state} label={running ? `${Math.round(progress)}%` : undefined} />
    </div>
  );
}
