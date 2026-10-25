import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';

export interface SectionTitleProps {
  children: string;
  /** A note at the far end of the row. */
  aside?: ReactNode;
  className?: string;
}

/** A side panel's small uppercase section heading. */
export default function SectionTitle({ children, aside, className }: SectionTitleProps) {
  return (
    <div className={cn('flex items-baseline justify-between', className)}>
      <h2 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
        {children}
      </h2>
      {aside ? <span className="text-meta text-fg-mute">{aside}</span> : null}
    </div>
  );
}
