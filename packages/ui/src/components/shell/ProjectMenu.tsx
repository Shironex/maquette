import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Monogram } from '../brand/Monogram';

export interface ProjectMenuProject {
  id: string;
  name: string;
  /** The second line: template, cards, age. */
  meta?: string;
  thumb?: string;
  /** The project open now. */
  current?: boolean;
  /** What runs in it, for example "Run 25 going". Shown with an amber dot. */
  live?: string;
}

export interface ProjectMenuAction {
  id: string;
  label: string;
  icon: IconName;
  onSelect: () => void;
}

export interface ProjectMenuProps {
  projects: readonly ProjectMenuProject[];
  /** New project, Import folder. */
  actions: readonly ProjectMenuAction[];
  onSelect: (id: string) => void;
  onClose: () => void;
  /** One line above the actions, for example what keeps running after a switch. */
  note?: string;
  /** The shortcut that opens the menu, shown in its header ("⌘P"). */
  shortcut?: string;
  /** The search field shows once there are more projects than this. */
  searchFrom?: number;
  /** Clicks inside it (the switcher) do not count as outside. */
  anchorRef?: RefObject<HTMLElement | null>;
  label?: string;
  className?: string;
}

const ITEM =
  'flex w-full cursor-pointer items-center gap-2.5 rounded-control px-2 text-left outline-none hover:bg-hover focus-visible:bg-hover focus-visible:shadow-[inset_0_0_0_1px_var(--focus)]';

function matches(project: ProjectMenuProject, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return `${project.name} ${project.meta ?? ''}`.toLowerCase().includes(q);
}

/**
 * The project switcher's popover: every project with its picture and meta,
 * the open one checked, a search field once the list is long, then New
 * project and Import folder. Arrow keys move, Home and End jump, Enter
 * picks, Escape closes and typing searches.
 */
export function ProjectMenu({
  projects,
  actions,
  onSelect,
  onClose,
  note,
  shortcut,
  searchFrom = 6,
  anchorRef,
  label = 'Projects',
  className,
}: ProjectMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');
  const searching = projects.length > searchFrom;
  const shown = useMemo(
    () => (searching ? projects.filter(p => matches(p, query)) : projects),
    [projects, query, searching]
  );

  const items = () => [...(ref.current?.querySelectorAll<HTMLElement>('[data-menu-item]') ?? [])];

  useEffect(() => {
    if (searching) search.current?.focus();
    else (ref.current?.querySelector<HTMLElement>('[aria-checked="true"]') ?? items()[0])?.focus();
    const onDown = (e: PointerEvent) => {
      const target = e.target as Node;
      if (ref.current?.contains(target) || anchorRef?.current?.contains(target)) return;
      onClose();
    };
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
    // Focus once on open; the list changing must not steal focus back.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const move = (to: 'next' | 'prev' | 'first' | 'last') => {
    const list = items();
    if (list.length === 0) return;
    const at = list.indexOf(document.activeElement as HTMLElement);
    const index =
      to === 'first'
        ? 0
        : to === 'last'
          ? list.length - 1
          : at < 0
            ? to === 'next'
              ? 0
              : list.length - 1
            : to === 'next'
              ? at + 1
              : at - 1;
    if (index < 0 && searching) {
      search.current?.focus();
      return;
    }
    list[(index + list.length) % list.length]?.focus();
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const inSearch = e.target === search.current;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      move('next');
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move('prev');
    } else if ((e.key === 'Home' || e.key === 'End') && !inSearch) {
      e.preventDefault();
      move(e.key === 'Home' ? 'first' : 'last');
    } else if (e.key === 'Enter' && inSearch) {
      e.preventDefault();
      const first = shown[0];
      if (first) onSelect(first.id);
    } else if (
      searching &&
      !inSearch &&
      e.key.length === 1 &&
      !e.metaKey &&
      !e.ctrlKey &&
      !e.altKey
    ) {
      // Typing on a row searches.
      e.preventDefault();
      setQuery(q => q + e.key);
      search.current?.focus();
    }
  };

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      onKeyDown={onKeyDown}
      className={cn(
        'flex w-[320px] flex-col rounded-card border border-line-strong bg-panel p-1.5 shadow-window',
        className
      )}
    >
      <div className="flex items-center justify-between px-2 pt-1 pb-1.5">
        <span className="text-caption tracking-[0.06em] text-fg-mute uppercase">
          {label} · {projects.length}
        </span>
        {shortcut ? (
          <kbd className="rounded-[5px] border border-line px-1.5 font-sans text-[11px] leading-[18px] text-fg-mute">
            {shortcut}
          </kbd>
        ) : null}
      </div>
      {searching ? (
        <label className="mx-1 mb-1.5 flex h-8 items-center gap-2 rounded-control border border-line bg-raised px-2.5 focus-within:border-focus">
          <Icon name="magnifying-glass" size={14} className="text-fg-mute" />
          <input
            ref={search}
            type="text"
            role="searchbox"
            aria-label="Find a project"
            placeholder="Find a project"
            spellCheck={false}
            value={query}
            onChange={e => setQuery(e.target.value)}
            className="min-w-0 flex-1 bg-transparent text-[13px] text-fg outline-none placeholder:text-fg-mute"
          />
        </label>
      ) : null}
      <div className="flex max-h-[340px] flex-col overflow-y-auto">
        {shown.map(p => (
          <button
            key={p.id}
            type="button"
            role="menuitemradio"
            aria-checked={p.current ? 'true' : 'false'}
            data-menu-item
            tabIndex={-1}
            onClick={() => onSelect(p.id)}
            className={cn(
              ITEM,
              'min-h-[50px] py-1.5',
              p.current && 'bg-clay-soft hover:bg-clay-soft'
            )}
          >
            {p.thumb ? (
              <img
                src={p.thumb}
                alt=""
                className="size-[34px] flex-none rounded-[7px] object-cover"
              />
            ) : (
              <span className="grid size-[34px] flex-none place-items-center rounded-[7px] bg-hover">
                <Monogram size={20} label="" />
              </span>
            )}
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-[13px] leading-4 font-semibold">{p.name}</span>
              {p.meta || p.live ? (
                <span className="flex min-w-0 items-center gap-1.5 text-[11.5px] leading-[15px] text-fg-mute">
                  {p.meta ? <span className="truncate">{p.meta}</span> : null}
                  {p.live ? (
                    <span className="inline-flex flex-none items-center gap-1 text-amber">
                      <span className="size-[6px] rounded-full bg-amber" />
                      {p.live}
                    </span>
                  ) : null}
                </span>
              ) : null}
            </span>
            {p.current ? (
              <Icon name="check" size={14} className="flex-none text-clay-text" />
            ) : null}
          </button>
        ))}
        {shown.length === 0 ? (
          <p className="px-2 py-3 text-meta text-fg-mute">{`No project matches "${query.trim()}".`}</p>
        ) : null}
      </div>
      {note ? (
        <p className="mx-1 mt-1.5 flex items-start gap-1.5 rounded-[7px] bg-raised px-2 py-1.5 text-meta text-fg-dim">
          <Icon name="info" size={14} className="mt-px flex-none text-amber" />
          {note}
        </p>
      ) : null}
      <div className="mt-1.5 flex flex-col border-t border-line pt-1.5">
        {actions.map(a => (
          <button
            key={a.id}
            type="button"
            role="menuitem"
            data-menu-item
            tabIndex={-1}
            onClick={a.onSelect}
            className={cn(ITEM, 'h-8 text-[13px] text-fg-dim hover:text-fg focus-visible:text-fg')}
          >
            <Icon name={a.icon} size={15} className="text-fg-mute" />
            {a.label}
          </button>
        ))}
      </div>
    </div>
  );
}
