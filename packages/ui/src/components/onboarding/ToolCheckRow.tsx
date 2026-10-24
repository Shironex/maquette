import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { StatusChip, type Status } from '../status/StatusChip';

export type ToolCheckStatus = 'found' | 'warning' | 'missing' | 'checking';

export interface ToolCheckRowProps {
  name: string;
  /** Marks the tool the app cannot run without; a missing one reads as failed, not a warning. */
  required?: boolean;
  status: ToolCheckStatus;
  /** One mono line: version and path, or why it is missing. */
  summary?: string;
  /** What to do next, under the summary. */
  hint?: string;
  action?: ReactNode;
  density?: 'roomy' | 'compact';
  className?: string;
}

function look(status: ToolCheckStatus, required: boolean) {
  const blocking = status === 'missing' && required;
  const icon: IconName =
    status === 'found'
      ? 'check-circle'
      : status === 'checking'
        ? 'circle-notch'
        : blocking
          ? 'x-circle'
          : 'warning';
  const tone =
    status === 'found'
      ? 'text-sage'
      : status === 'checking'
        ? 'text-fg-mute'
        : blocking
          ? 'text-danger'
          : 'text-amber';
  const chip: Status =
    status === 'found'
      ? 'done'
      : status === 'checking'
        ? 'queued'
        : blocking
          ? 'failed'
          : 'running';
  const word =
    status === 'found'
      ? 'Found'
      : status === 'checking'
        ? 'Checking'
        : status === 'warning'
          ? 'Check'
          : 'Missing';
  return { icon, tone, chip, word };
}

export function ToolCheckRow({
  name,
  required = false,
  status,
  summary,
  hint,
  action,
  density = 'roomy',
  className,
}: ToolCheckRowProps) {
  const l = look(status, required);
  const roomy = density === 'roomy';
  return (
    <div
      className={cn(
        'flex gap-3 border-t border-line first:border-t-0',
        roomy ? 'items-start px-4 py-3' : 'items-center px-4 py-2',
        className
      )}
    >
      <Icon
        name={l.icon}
        size={roomy ? 16 : 14}
        spin={status === 'checking'}
        title={l.word}
        className={cn(l.tone, roomy && 'mt-0.5')}
      />
      <div className={cn('flex-none font-semibold', roomy ? 'w-[150px]' : 'w-28')}>
        {name}
        {required && roomy ? (
          <StatusChip status="queued" label="Required" icon={false} className="ml-1 align-[1px]" />
        ) : null}
      </div>
      <div className="min-w-0 flex-1">
        {summary ? (
          <div
            title={summary}
            className={cn(
              'truncate font-mono text-[12px] leading-4 tracking-[-0.01em]',
              status === 'found' ? (roomy ? 'text-fg-dim' : 'text-fg-mute') : l.tone,
              roomy && 'pt-0.5'
            )}
          >
            {summary}
          </div>
        ) : null}
        {hint && roomy ? <div className="mt-[3px] text-meta text-fg-mute">{hint}</div> : null}
      </div>
      {roomy ? <StatusChip status={l.chip} label={l.word} icon={false} /> : null}
      {action ?? null}
    </div>
  );
}
