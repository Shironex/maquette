import { cn } from '@maquette/ui';
import { PANEL_TITLE } from './panelTitle';
import type { PanelProps } from './Panel.types';

export default function Panel({
  title,
  help,
  helpInline,
  actions,
  id,
  className,
  children,
}: PanelProps) {
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
