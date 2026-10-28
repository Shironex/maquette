import { FilterList, FilterGroup, FilterOption } from '@/features/common/FilterList';
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

/**
 * Tags, what made each model, its format and the projects that use it.
 * Picking an option again clears it; the counts are over the whole library.
 */
export default function LibraryFilters({ items, filters, onChange }: LibraryFiltersProps) {
  const toggle = <K extends keyof Filters>(key: K, value: Filters[K]) =>
    onChange({ [key]: filters[key] === value ? null : value } as Partial<Filters>);
  const tags = tagCounts(items).map(([tag, n]) => (
    <FilterOption
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
      <FilterOption
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
      <FilterOption
        key={format}
        label={`.${format}`}
        count={n}
        on={filters.format === format}
        onClick={() => toggle('format', format)}
      />
    ));
  const projects = projectCounts(items).map(([id, name, n]) => (
    <FilterOption
      key={id}
      label={name}
      count={n}
      on={filters.usedIn === id}
      onClick={() => toggle('usedIn', id)}
    />
  ));
  const unused = items.filter(i => i.uses.length === 0).length;

  return (
    <FilterList>
      <FilterGroup title="Tags">
        <FilterOption
          label="All models"
          count={items.length}
          on={!filters.tag && !filters.source && !filters.format && filters.usedIn === null}
          onClick={() => onChange({ tag: null, source: null, format: null, usedIn: null })}
        />
        {tags}
      </FilterGroup>
      <FilterGroup title="Made with">{sources}</FilterGroup>
      {formats.length > 1 ? <FilterGroup title="Format">{formats}</FilterGroup> : null}
      <FilterGroup title="Used in">
        {projects}
        <FilterOption
          label="Not used"
          count={unused}
          on={filters.usedIn === NOT_USED}
          onClick={() => toggle('usedIn', NOT_USED)}
        />
      </FilterGroup>
    </FilterList>
  );
}
