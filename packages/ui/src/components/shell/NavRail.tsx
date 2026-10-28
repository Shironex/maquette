import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Tooltip } from '../feedback/Tooltip';
import { ProjectSwitcher, type ProjectSwitcherProps } from './ProjectSwitcher';

export interface NavRailItem<T extends string = string> {
  id: T;
  label: string;
  icon: IconName;
  /** A count, as the clay pill (Inbox). */
  badge?: number;
  /** A live marker, as the amber dot (a running pipeline). */
  live?: boolean;
  /** A plain count after the label, for example a library's size. */
  count?: number;
}

/** Views under a heading of their own, for example Libraries. */
export interface NavRailGroup<T extends string = string> {
  title: string;
  items: readonly NavRailItem<T>[];
}

/** A view that is not built yet: shown greyed out, never selectable. */
export interface NavRailPlanned {
  id: string;
  label: string;
  icon: IconName;
  /** A short tag after the label, for example "later". */
  tag?: string;
}

export interface NavRailProps<T extends string> {
  items: readonly NavRailItem<T>[];
  /** More views under their own headings, after the tools. */
  groups?: readonly NavRailGroup<T>[];
  /** Views that are not built yet, under a Planned heading. */
  planned?: readonly NavRailPlanned[];
  /** The bottom item, Settings. */
  footer: NavRailItem<T>;
  value: T;
  onChange: (id: T) => void;
  project?: ProjectSwitcherProps;
  /** Shown under the switcher, for example its project menu. */
  projectMenu?: ReactNode;
  /** The lights row and the logo, above the switcher. */
  header?: ReactNode;
  /** Icons only: the labels move to tooltips and the switcher to its picture. */
  collapsed?: boolean;
  /** Shows the collapse button at the bottom. */
  onToggleCollapsed?: () => void;
  className?: string;
}

const RAIL_BUTTON =
  'group relative flex h-[34px] w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-left font-medium text-fg-dim transition-colors duration-120 hover:bg-hover hover:text-fg active:bg-line';

function RailButton<T extends string>({
  item,
  current,
  collapsed,
  onChange,
}: {
  item: NavRailItem<T>;
  current: boolean;
  collapsed?: boolean;
  onChange: (id: T) => void;
}) {
  const extra = item.badge
    ? `, ${item.badge} new`
    : item.live
      ? ', running'
      : item.count !== undefined
        ? `, ${item.count}`
        : '';
  return (
    <Tooltip label={item.label} side="right" disabled={!collapsed}>
      <button
        type="button"
        aria-current={current ? 'page' : undefined}
        aria-label={collapsed ? `${item.label}${extra}` : undefined}
        onClick={() => onChange(item.id)}
        className={cn(
          RAIL_BUTTON,
          collapsed && 'justify-center px-0',
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
        {collapsed ? (
          item.badge || item.live ? (
            <span
              aria-hidden="true"
              className={cn(
                'absolute top-1.5 right-2.5 size-[7px] rounded-full',
                item.badge ? 'bg-clay' : 'bg-amber'
              )}
            />
          ) : null
        ) : (
          <RailLabel item={item} />
        )}
      </button>
    </Tooltip>
  );
}

function RailLabel<T extends string>({ item }: { item: NavRailItem<T> }) {
  return (
    <>
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
      ) : item.count !== undefined ? (
        <span className="ml-auto font-mono text-[11px] text-fg-mute">{item.count}</span>
      ) : null}
    </>
  );
}

function Heading({ title, collapsed }: { title: string; collapsed?: boolean }) {
  return collapsed ? (
    <div className="mx-2 mt-2 mb-1.5 border-t border-line" />
  ) : (
    <div className="px-2 pt-3 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
      {title}
    </div>
  );
}

function PlannedRow({ item, collapsed }: { item: NavRailPlanned; collapsed?: boolean }) {
  const label = item.tag ? `${item.label}, ${item.tag}` : item.label;
  return (
    <Tooltip label={`${label}: not built yet`} side="right">
      <div
        className={cn(
          'flex h-[34px] w-full items-center gap-2.5 rounded-control px-2.5 font-medium text-fg-mute',
          collapsed && 'justify-center px-0'
        )}
      >
        <Icon name={item.icon} className="text-fg-mute" />
        <span className="sr-only">{collapsed ? `${label}, ` : ''}not built yet</span>
        {collapsed ? null : (
          <>
            {item.label}
            {item.tag ? (
              <span className="ml-auto rounded-pill border border-line px-1.5 text-[11px] leading-[16px] text-fg-mute">
                {item.tag}
              </span>
            ) : null}
          </>
        )}
      </div>
    </Tooltip>
  );
}

export function NavRail<T extends string>({
  items,
  groups,
  planned,
  footer,
  value,
  onChange,
  project,
  projectMenu,
  header,
  collapsed,
  onToggleCollapsed,
  className,
}: NavRailProps<T>) {
  const toggle = collapsed ? 'Expand the sidebar' : 'Collapse the sidebar';
  const plannedRows = planned?.map(p => <PlannedRow key={p.id} item={p} collapsed={collapsed} />);
  const buttons = (list: readonly NavRailItem<T>[]) =>
    list.map(it => (
      <RailButton
        key={it.id}
        item={it}
        current={it.id === value}
        collapsed={collapsed}
        onChange={onChange}
      />
    ));
  const groupBlocks = groups?.map(g => (
    <div key={g.title} role="group" aria-label={g.title} className="flex flex-col gap-1">
      <Heading title={g.title} collapsed={collapsed} />
      {buttons(g.items)}
    </div>
  ));
  return (
    <nav
      aria-label="Main"
      className={cn(
        'flex flex-none flex-col gap-1 border-r border-line bg-panel pt-4 pb-3 transition-[width] duration-140',
        collapsed ? 'w-[76px] px-3' : 'w-52 px-3',
        className
      )}
    >
      {header}
      {project ? (
        <div className="relative mb-3.5">
          <ProjectSwitcher {...project} compact={collapsed || project.compact} />
          {projectMenu}
        </div>
      ) : null}
      {collapsed ? (
        <div className="mx-2 mt-1 mb-1.5 border-t border-line" />
      ) : (
        <div className="px-2 pt-2 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
          Tools
        </div>
      )}
      {buttons(items)}
      {groupBlocks}
      {planned?.length ? (
        <>
          <Heading title="Planned" collapsed={collapsed} />
          {plannedRows}
        </>
      ) : null}
      <div className="flex-1" />
      <RailButton
        item={footer}
        current={footer.id === value}
        collapsed={collapsed}
        onChange={onChange}
      />
      {onToggleCollapsed ? (
        <Tooltip label={toggle} side="right" disabled={!collapsed}>
          <button
            type="button"
            aria-label={toggle}
            aria-expanded={!collapsed}
            onClick={onToggleCollapsed}
            className={cn(RAIL_BUTTON, 'h-[30px] text-fg-mute', collapsed && 'justify-center px-0')}
          >
            <Icon name="sidebar-simple" className="text-fg-mute group-hover:text-fg-dim" />
            {collapsed ? null : <span className="text-meta">Collapse</span>}
          </button>
        </Tooltip>
      ) : null}
    </nav>
  );
}
