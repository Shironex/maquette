import { cn } from '../../lib/cn';

export type ProgressState = 'running' | 'done' | 'failed' | 'idle';

export interface ProgressBarProps {
  /** From 0 to 100. Leave it out when the job does not report how far it is. */
  value?: number;
  state?: ProgressState;
  label?: string;
  valueLabel?: string;
  className?: string;
}

const FILL: Record<ProgressState, string> = {
  running: 'bg-amber',
  done: 'bg-sage',
  failed: 'bg-danger',
  idle: 'bg-line-strong',
};

export function ProgressBar({
  value,
  state = 'running',
  label,
  valueLabel,
  className,
}: ProgressBarProps) {
  const known = value !== undefined && Number.isFinite(value);
  const v = known ? Math.max(0, Math.min(100, value)) : 0;
  return (
    <div className={cn('flex min-w-30 flex-col gap-1.5', className)}>
      {label ? (
        <div className="flex justify-between text-meta text-fg-dim">
          <span>{label}</span>
          <span className="text-fg-mute tabular-nums">
            {valueLabel ?? (known ? `${Math.round(v)}%` : '')}
          </span>
        </div>
      ) : null}
      <div
        role="progressbar"
        aria-valuenow={known ? Math.round(v) : undefined}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label ?? 'Progress'}
        aria-busy={!known || undefined}
        className="h-1 overflow-hidden rounded-pill bg-hover"
      >
        {known ? (
          <div
            className={cn(
              'h-full rounded-pill transition-[width] duration-300 ease-out',
              FILL[state]
            )}
            style={{ width: `${v}%` }}
          />
        ) : (
          <div
            data-indeterminate=""
            className={cn('h-full w-2/5 animate-indeterminate rounded-pill', FILL[state])}
          />
        )}
      </div>
    </div>
  );
}
