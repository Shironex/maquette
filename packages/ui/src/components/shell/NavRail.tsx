import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { ProjectSwitcher, type ProjectSwitcherProps } from './ProjectSwitcher';

export interface NavRailItem<T extends string = string> {
  id: T;
  label: string;
  icon: IconName;
  /** A count, as the clay pill (Inbox). */
  badge?: number;
  /** A live marker, as the amber dot (a running pipeline). */
  live?: boolean;
}

export interface NavRailProps<T extends string> {
  items: readonly NavRailItem<T>[];
  /** The bottom item, Settings. */
  footer: NavRailItem<T>;
  value: T;
  onChange: (id: T) => void;
  project?: ProjectSwitcherProps;
  /** The lights row and the logo, above the switcher. */
  header?: ReactNode;
  label?: string;
  className?: string;
}

function RailButton<T extends string>({
  item,
  current,
  onChange,
}: {
  item: NavRailItem<T>;
  current: boolean;
  onChange: (id: T) => void;
}) {
  return (
    <button
      type="button"
      aria-current={current ? 'page' : undefined}
      onClick={() => onChange(item.id)}
      className={cn(
        'group flex h-[34px] w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-left font-medium text-fg-dim transition-colors duration-120 hover:bg-hover hover:text-fg',
        current && 'bg-clay-soft text-fg hover:bg-clay-soft'
      )}
    >
      <Icon
        name={item.icon}
        className={cn(
          'text-fg-mute group-hover:text-fg-dim',
          current && 'text-clay-text group-hover:text-clay-text'
        )}
      />
      {item.label}
      {item.badge ? (
        <span className="ml-auto rounded-pill bg-clay px-[7px] font-mono text-[11px] leading-[18px] font-semibold text-clay-ink">
          {item.badge}
          <span className="sr-only"> new</span>
        </span>
      ) : item.live ? (
        <span className="ml-auto size-[7px] rounded-full bg-amber shadow-[0_0_0_3px_var(--amber-soft)]">
          <span className="sr-only">running</span>
        </span>
      ) : null}
    </button>
  );
}

export function NavRail<T extends string>({
  items,
  footer,
  value,
  onChange,
  project,
  header,
  label = 'Main',
  className,
}: NavRailProps<T>) {
  return (
    <nav
      aria-label={label}
      className={cn(
        'flex w-52 flex-none flex-col gap-1 border-r border-line bg-panel px-3 pt-4 pb-3',
        className
      )}
    >
      {header}
      {project ? <ProjectSwitcher {...project} className="mb-3.5" /> : null}
      <div className="px-2 pt-2 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
        Tools
      </div>
      {items.map(it => (
        <RailButton key={it.id} item={it} current={it.id === value} onChange={onChange} />
      ))}
      <div className="flex-1" />
      <RailButton item={footer} current={footer.id === value} onChange={onChange} />
    </nav>
  );
}
