import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { ProgressBar } from '../status/ProgressBar';
import { StatusChip, type Status } from '../status/StatusChip';

export type NodeState = 'cached' | 'done' | 'running' | 'failed' | 'waiting';

const CHIP: Record<NodeState, Status> = {
  cached: 'cached',
  done: 'done',
  running: 'running',
  failed: 'failed',
  waiting: 'queued',
};

export interface PipelineNodeProps {
  step: number;
  name: string;
  icon?: IconName;
  script?: string;
  state?: NodeState;
  progress?: number;
  duration?: string;
  detail?: string;
  /** Replaces the chip's word, for example "failed in 24". */
  label?: string;
  /** Small actions after the duration, for example Run from here. */
  actions?: ReactNode;
  className?: string;
}

export function PipelineNode({
  step,
  name,
  icon,
  script,
  state = 'waiting',
  progress,
  duration,
  detail,
  label,
  actions,
  className,
}: PipelineNodeProps) {
  return (
    <div
      className={cn(
        '@container relative flex min-w-0 flex-col gap-2 rounded-card border border-line bg-raised px-3 pt-[11px] pb-3',
        state === 'running' && 'border-amber/60 shadow-running',
        state === 'failed' && 'border-danger',
        className
      )}
    >
      <div className="flex items-center gap-2">
        <span
          className={cn(
            'grid size-5 flex-none place-items-center rounded-control bg-hover font-mono text-[11px] leading-none font-semibold text-fg-mute',
            state === 'running' && 'bg-amber-soft text-amber',
            state === 'cached' && 'bg-sage-soft text-sage'
          )}
        >
          {step}
        </span>
        <span title={name} className="flex-1 truncate font-semibold">
          {name}
        </span>
        {/* A narrow node gives the step name the icon's room. */}
        {icon ? <Icon name={icon} className="text-fg-dim @max-[11.5rem]:hidden" /> : null}
      </div>
      {script ? (
        <div
          title={script}
          className="truncate font-mono text-[11.5px] leading-[15px] text-fg-mute"
        >
          {script}
        </div>
      ) : null}
      {state === 'running' && progress !== undefined ? <ProgressBar value={progress} /> : null}
      <div className="flex items-center justify-between gap-2">
        <StatusChip
          status={CHIP[state]}
          label={label ?? (state === 'waiting' ? 'Waiting' : undefined)}
        />
        <span className="flex items-center gap-1">
          {duration ? (
            <span className="text-meta whitespace-nowrap text-fg-mute tabular-nums">
              {duration}
            </span>
          ) : null}
          {actions ?? null}
        </span>
      </div>
      {detail ? <div className="text-meta text-danger">{detail}</div> : null}
    </div>
  );
}
