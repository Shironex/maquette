import { FilterList, FilterGroup, FilterOption } from '@/features/common/FilterList';
import type { LibraryCollection, LibraryItem } from '@/ipc/client';
import {
  NOT_USED,
  SOURCE_LABEL,
  UNTAGGED,
  formatCounts,
  projectCounts,
  sourceCounts,
  tagCounts,
  type LibraryFilters as Filters,
} from '@/state/library';
import { NewCollection } from '../NewCollection';

/** The dots that tell collections apart, in turn. */
const DOTS = ['bg-clay', 'bg-sage', 'bg-amber', 'bg-fg-dim', 'bg-danger'];

export interface LibraryFiltersProps {
  items: readonly LibraryItem[];
  filters: Filters;
  onChange: (change: Partial<Filters>) => void;
  collections?: readonly LibraryCollection[];
  /** The bulk selection, which a new collection starts with. */
  selected?: readonly string[];
}

/**
 * Collections, tags (models with none called out), what made each model,
 * its format and the projects that use it. Picking an option again clears
 * it; the counts are over the whole library.
 */
export default function LibraryFilters({
  items,
  filters,
  onChange,
  collections = [],
  selected = [],
}: LibraryFiltersProps) {
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
  const untagged = items.filter(i => i.entry.tags.length === 0).length;
  const sets = collections.map((c, n) => (
    <FilterOption
      key={c.id}
      label={
        <span className="flex items-center gap-2">
          <span
            aria-hidden="true"
            className={`size-2 flex-none rounded-full ${DOTS[n % DOTS.length]}`}
          />
          <span className="truncate">{c.name}</span>
        </span>
      }
      count={c.entries.length}
      on={filters.collection === c.id}
      onClick={() => toggle('collection', c.id)}
    />
  ));
  const anyFilter = filters.collection || filters.tag !== null || filters.source || filters.format;

  return (
    <FilterList>
      <FilterGroup title="Collections">
        <FilterOption
          label="All models"
          count={items.length}
          on={!anyFilter && filters.usedIn === null}
          onClick={() =>
            onChange({ collection: null, tag: null, source: null, format: null, usedIn: null })
          }
        />
        {sets}
        <NewCollection entries={selected} />
      </FilterGroup>
      <FilterGroup title="Tags">
        {tags}
        {untagged ? (
          <FilterOption
            label={<span className="text-amber">untagged</span>}
            count={untagged}
            on={filters.tag === UNTAGGED}
            onClick={() => toggle('tag', UNTAGGED)}
          />
        ) : null}
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
