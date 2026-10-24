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
  className,
}: PipelineNodeProps) {
  return (
    <div
      className={cn(
        'relative flex min-w-0 flex-col gap-2 rounded-card border border-line bg-raised px-3 pt-[11px] pb-3',
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
        <span className="flex-1 truncate font-semibold">{name}</span>
        {icon ? <Icon name={icon} className="text-fg-dim" /> : null}
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
        <StatusChip status={CHIP[state]} label={state === 'waiting' ? 'Waiting' : undefined} />
        {duration ? (
          <span className="text-meta whitespace-nowrap text-fg-mute tabular-nums">{duration}</span>
        ) : null}
      </div>
      {detail ? <div className="text-meta text-danger">{detail}</div> : null}
    </div>
  );
}
