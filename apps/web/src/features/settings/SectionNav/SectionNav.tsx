import type { ReactNode } from 'react';
import { Icon, cn } from '@maquette/ui';
import { SECTIONS, type SectionId } from '../sections';
import { useSectionNav } from './SectionNav.hooks';
import type { SectionNavProps } from './SectionNav.types';

export default function SectionNav({ active, onSelect }: SectionNavProps) {
  const { checked, summary, project, thumb } = useSectionNav();
  const navItem = (id: SectionId, label: string, lead: ReactNode, meta?: ReactNode) => (
    <button
      key={id}
      type="button"
      aria-current={active === id ? 'true' : undefined}
      onClick={() => onSelect(id)}
      className={cn(
        'flex h-[34px] w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-left font-medium text-fg-dim hover:bg-hover hover:text-fg',
        active === id && 'bg-clay-soft text-fg hover:bg-clay-soft [&>svg]:text-clay-text'
      )}
    >
      {lead}
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {meta}
    </button>
  );
  const sections = SECTIONS.map(s =>
    navItem(
      s.id,
      s.label,
      <Icon name={s.icon} className={active === s.id ? 'text-clay-text' : 'text-fg-mute'} />,
      s.id === 'toolchain' && checked ? (
        <span
          className={cn(
            'font-mono text-[11px]',
            summary.warnings || summary.blocked ? 'text-amber' : 'text-fg-mute'
          )}
        >
          {summary.found} of {summary.total}
        </span>
      ) : undefined
    )
  );
  return (
    <nav
      aria-label="Settings sections"
      className="flex w-[232px] flex-none flex-col gap-0.5 border-r border-line px-3 py-[18px]"
    >
      {sections}
      {project ? (
        <>
          <div className="px-2 pt-[22px] pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
            This project
          </div>
          {navItem(
            'project',
            project.name,
            thumb ? (
              <img src={thumb} alt="" className="size-4 rounded-[4px] object-cover" />
            ) : (
              <Icon name="cube" className="text-fg-mute" />
            )
          )}
        </>
      ) : null}
    </nav>
  );
}
