import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

export interface TooltipProps {
  label: string;
  shortcut?: string;
  side?: 'top' | 'bottom';
  open?: boolean;
  /** Wraps a longer sentence at a fixed width instead of one line. */
  wrap?: boolean;
  children: ReactNode;
  className?: string;
}

/** A hover and focus label. The trigger keeps its own accessible name. */
export function Tooltip({
  label,
  shortcut,
  side = 'top',
  open,
  wrap,
  children,
  className,
}: TooltipProps) {
  return (
    <span className={cn('group/tip relative inline-flex', className)}>
      {children}
      <span
        role="tooltip"
        className={cn(
          'pointer-events-none absolute left-1/2 z-10 -translate-x-1/2 rounded-control bg-fg px-2 py-[5px] text-[12px] leading-4 font-medium text-app opacity-0 shadow-raised transition-[opacity,translate] duration-120 ease-out',
          side === 'top'
            ? 'bottom-[calc(100%+8px)] translate-y-1'
            : 'top-[calc(100%+8px)] -translate-y-1',
          'group-hover/tip:translate-y-0 group-hover/tip:opacity-100 group-focus-within/tip:translate-y-0 group-focus-within/tip:opacity-100',
          wrap ? 'w-max max-w-[260px] whitespace-normal' : 'whitespace-nowrap',
          open && 'translate-y-0 opacity-100'
        )}
      >
        {label}
        {shortcut ? (
          <kbd className="ml-1.5 font-mono text-[11px] text-hover">{shortcut}</kbd>
        ) : null}
      </span>
    </span>
  );
}
