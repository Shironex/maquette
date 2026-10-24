import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

export interface TopBarProps {
  /** The left side: the stage stepper, or a title with its breadcrumb. */
  lead?: ReactNode;
  center?: ReactNode;
  /** The budget meter and the one primary action. */
  right?: ReactNode;
  className?: string;
}

export function TopBar({ lead, center, right, className }: TopBarProps) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        'flex h-14 flex-none items-center gap-4 border-b border-line bg-app pr-5 pl-6',
        className
      )}
    >
      {lead ? <div className="flex min-w-0 items-center gap-3">{lead}</div> : null}
      {center ? <div className="flex min-w-0 flex-1 justify-center">{center}</div> : null}
      {right ? <div className="ml-auto flex flex-none items-center gap-5">{right}</div> : null}
    </header>
  );
}
