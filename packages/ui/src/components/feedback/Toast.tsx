import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { IconButton } from '../actions/IconButton';

export type ToastStatus = 'done' | 'running' | 'failed' | 'info';

const ICON = {
  done: 'check-circle',
  running: 'circle-notch',
  failed: 'x-circle',
  info: 'info',
} as const;
const TONE = {
  done: 'text-sage',
  running: 'text-amber',
  failed: 'text-danger',
  info: 'text-fg-dim',
} as const;

export interface ToastProps {
  status?: ToastStatus;
  title: string;
  detail?: string;
  action?: ReactNode;
  onClose?: () => void;
  className?: string;
}

export function Toast({ status = 'info', title, detail, action, onClose, className }: ToastProps) {
  return (
    <div
      role={status === 'failed' ? 'alert' : 'status'}
      className={cn(
        'grid w-[360px] grid-cols-[20px_1fr_auto] items-start gap-3 rounded-card border border-line-strong bg-panel p-3 shadow-raised',
        className
      )}
    >
      <Icon
        name={ICON[status]}
        size={18}
        spin={status === 'running'}
        className={cn('mt-px', TONE[status])}
      />
      <div>
        <div className="font-medium text-fg">{title}</div>
        {detail ? (
          <div className="mt-0.5 text-[12px] leading-[17px] text-fg-dim">{detail}</div>
        ) : null}
      </div>
      <div className="flex items-center gap-1">
        {action ?? null}
        {onClose ? <IconButton icon="x" label="Dismiss" size="sm" onClick={onClose} /> : null}
      </div>
    </div>
  );
}
