import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import {
  NOT_USED,
  SOURCE_LABEL,
  formatCounts,
  projectCounts,
  sourceCounts,
  tagCounts,
  type LibraryFilters as Filters,
} from '@/state/library';

export interface LibraryFiltersProps {
  items: readonly LibraryItem[];
  filters: Filters;
  onChange: (change: Partial<Filters>) => void;
}

function Option({
  label,
  count,
  on,
  onClick,
}: {
  label: ReactNode;
  count: number;
  on: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onClick}
      className={cn(
        'flex h-8 w-full cursor-pointer items-center gap-2 rounded-control px-2.5 text-left text-fg-dim hover:bg-hover hover:text-fg',
        on && 'bg-clay-soft font-medium text-fg hover:bg-clay-soft'
      )}
    >
      <span className="min-w-0 flex-1 truncate">{label}</span>
      <span className="font-mono text-[11px] text-fg-mute">{count}</span>
    </button>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div role="group" aria-label={title} className="flex flex-col gap-0.5">
      <div className="px-2.5 pt-1 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
        {title}
      </div>
      {children}
    </div>
  );
}

/**
 * Tags, what made each model, its format and the projects that use it.
 * Picking an option again clears it; the counts are over the whole library.
 */
export default function LibraryFilters({ items, filters, onChange }: LibraryFiltersProps) {
  const toggle = <K extends keyof Filters>(key: K, value: Filters[K]) =>
    onChange({ [key]: filters[key] === value ? null : value } as Partial<Filters>);
  const tags = tagCounts(items).map(([tag, n]) => (
    <Option
      key={tag}
      label={tag}
      count={n}
      on={filters.tag === tag}
      onClick={() => toggle('tag', tag)}
    />
  ));
  const sources = sourceCounts(items)
    .filter(([, n]) => n > 0)
    .map(([source, n]) => (
      <Option
        key={source}
        label={SOURCE_LABEL[source]}
        count={n}
        on={filters.source === source}
        onClick={() => toggle('source', source)}
      />
    ));
  const formats = formatCounts(items)
    .filter(([, n]) => n > 0)
    .map(([format, n]) => (
      <Option
        key={format}
        label={`.${format}`}
        count={n}
        on={filters.format === format}
        onClick={() => toggle('format', format)}
      />
    ));
  const projects = projectCounts(items).map(([id, name, n]) => (
    <Option
      key={id}
      label={name}
      count={n}
      on={filters.usedIn === id}
      onClick={() => toggle('usedIn', id)}
    />
  ));
  const unused = items.filter(i => i.uses.length === 0).length;

  return (
    <section
      aria-label="Filters"
      className="flex w-[232px] flex-none flex-col gap-5 overflow-y-auto border-r border-line bg-panel px-3 py-[18px]"
    >
      <Group title="Tags">
        <Option
          label="All models"
          count={items.length}
          on={!filters.tag && !filters.source && !filters.format && filters.usedIn === null}
          onClick={() => onChange({ tag: null, source: null, format: null, usedIn: null })}
        />
        {tags}
      </Group>
      <Group title="Made with">{sources}</Group>
      {formats.length > 1 ? <Group title="Format">{formats}</Group> : null}
      <Group title="Used in">
        {projects}
        <Option
          label="Not used"
          count={unused}
          on={filters.usedIn === NOT_USED}
          onClick={() => toggle('usedIn', NOT_USED)}
        />
      </Group>
    </section>
  );
}
