import { cn } from '../../lib/cn';

export type ProgressState = 'running' | 'done' | 'failed' | 'idle';

export interface ProgressBarProps {
  value: number;
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
  const v = Math.max(0, Math.min(100, value));
  return (
    <div className={cn('flex min-w-30 flex-col gap-1.5', className)}>
      {label ? (
        <div className="flex justify-between text-meta text-fg-dim">
          <span>{label}</span>
          <span className="text-fg-mute tabular-nums">{valueLabel ?? `${Math.round(v)}%`}</span>
        </div>
      ) : null}
      <div
        role="progressbar"
        aria-valuenow={Math.round(v)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label ?? 'Progress'}
        className="h-1 overflow-hidden rounded-pill bg-hover"
      >
        <div
          className={cn(
            'h-full rounded-pill transition-[width] duration-300 ease-out',
            FILL[state]
          )}
          style={{ width: `${v}%` }}
        />
      </div>
    </div>
  );
}
