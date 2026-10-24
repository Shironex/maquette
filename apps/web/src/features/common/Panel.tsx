import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';

/** The canvas `title-m`: 15/20 semibold. */
export const PANEL_TITLE = 'text-emphasis leading-5 font-semibold tracking-[-0.01em]';

export interface PanelProps {
  title?: string;
  /** A short line next to or under the title. */
  help?: string;
  helpInline?: boolean;
  actions?: ReactNode;
  id?: string;
  className?: string;
  children: ReactNode;
}

export function Panel({ title, help, helpInline, actions, id, className, children }: PanelProps) {
  const headingId = id ? `${id}-title` : undefined;
  return (
    <section
      id={id}
      aria-labelledby={headingId}
      className={cn(
        'flex flex-col gap-3.5 rounded-card border border-line bg-panel p-4',
        className
      )}
    >
      {title ? (
        <div className={cn('flex gap-2.5', helpInline ? 'items-baseline' : 'items-start')}>
          <div className={cn('min-w-0 flex-1', helpInline && 'flex items-baseline gap-2.5')}>
            <h2 id={headingId} className={PANEL_TITLE}>
              {title}
            </h2>
            {help ? (
              <p className={cn('text-meta text-fg-mute', !helpInline && 'mt-0.5')}>{help}</p>
            ) : null}
          </div>
          {actions}
        </div>
      ) : null}
      {children}
    </section>
  );
}

export function SectionLabel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <p className={cn('text-caption tracking-[0.04em] text-fg-mute uppercase', className)}>
      {children}
    </p>
  );
}
