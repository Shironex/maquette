import { Icon, cn } from '@maquette/ui';
import type { ProjectTemplate } from '@/ipc/client';
import { TEMPLATES } from './templates';

export interface TemplateGridProps {
  value: ProjectTemplate;
  compact?: boolean;
  onChange: (template: Exclude<ProjectTemplate, 'imported'>) => void;
}

export default function TemplateGrid({ value, compact, onChange }: TemplateGridProps) {
  const cards = TEMPLATES.map(t => {
    const on = t.id === value;
    return (
      <button
        key={t.id}
        type="button"
        aria-pressed={on}
        onClick={() => onChange(t.id)}
        className={cn(
          'flex cursor-pointer flex-col overflow-hidden rounded-card border bg-raised text-left transition-colors duration-120',
          on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
        )}
      >
        <div className={cn('relative bg-[#7f8083]', compact ? 'h-[88px]' : 'h-[156px]')}>
          <img
            src={t.image}
            alt=""
            className={cn(
              'block size-full',
              t.contain ? 'object-contain brightness-[0.84] saturate-[1.04]' : 'object-cover'
            )}
          />
          {on ? (
            <span className="absolute top-2 left-2 inline-flex h-[22px] items-center gap-1.5 rounded-[6px] bg-clay px-2 text-[11.5px] font-medium text-clay-ink">
              <Icon name="check" size={12} />
              Selected
            </span>
          ) : null}
        </div>
        <div className={compact ? 'px-2.5 pt-2 pb-2.5' : 'px-3 pt-2.5 pb-3'}>
          <div className={cn('font-semibold', compact && 'text-[12.5px]')}>{t.title}</div>
          {compact ? null : <div className="mt-0.5 text-meta text-fg-mute">{t.detail}</div>}
        </div>
      </button>
    );
  });
  return (
    <div
      className={cn('mt-4 grid grid-cols-3', compact ? 'gap-2.5' : 'gap-3.5')}
      role="group"
      aria-label="Templates"
    >
      {cards}
    </div>
  );
}
