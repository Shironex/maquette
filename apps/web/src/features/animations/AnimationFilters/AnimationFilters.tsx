import { FilterList, FilterGroup, FilterOption } from '@/features/common/FilterList';
import type { AnimationItem } from '@/ipc/client';
import {
  RIG_LABEL,
  packCounts,
  rigCounts,
  tagCounts,
  type AnimationFilters as Filters,
} from '@/state/animations';

export interface AnimationFiltersProps {
  items: readonly AnimationItem[];
  filters: Filters;
  onChange: (change: Partial<Filters>) => void;
  /** Where the library keeps its clips. */
  dir: string | null;
}

/**
 * Packs, tags and the rig each clip was made on. Picking an option again
 * clears it; the counts are over the whole library.
 */
export default function AnimationFilters({ items, filters, onChange, dir }: AnimationFiltersProps) {
  const toggle = <K extends 'pack' | 'tag' | 'rig'>(key: K, value: Filters[K]) =>
    onChange({ [key]: filters[key] === value ? null : value } as Partial<Filters>);
  const packs = packCounts(items).map(([pack, n]) => (
    <FilterOption
      key={pack}
      label={pack}
      count={n}
      on={filters.pack === pack}
      onClick={() => toggle('pack', pack)}
    />
  ));
  const tags = tagCounts(items).map(([tag, n]) => (
    <FilterOption
      key={tag}
      label={tag}
      count={n}
      on={filters.tag === tag}
      onClick={() => toggle('tag', tag)}
    />
  ));
  const rigs = rigCounts(items)
    .filter(([, n]) => n > 0)
    .map(([rig, n]) => (
      <FilterOption
        key={rig}
        label={RIG_LABEL[rig]}
        count={n}
        on={filters.rig === rig}
        onClick={() => toggle('rig', rig)}
      />
    ));

  return (
    <FilterList>
      <FilterGroup title="Packs">
        <FilterOption
          label="All clips"
          count={items.length}
          on={!filters.pack && !filters.tag && !filters.rig}
          onClick={() => onChange({ pack: null, tag: null, rig: null })}
        />
        {packs}
      </FilterGroup>
      {tags.length ? <FilterGroup title="Tags">{tags}</FilterGroup> : null}
      <FilterGroup title="Source rig">{rigs}</FilterGroup>
      {dir ? (
        <p className="mt-auto px-2.5 font-mono text-[11px] leading-4 break-all text-fg-mute">
          {dir} <span className="text-sage">local</span>
        </p>
      ) : null}
    </FilterList>
  );
}
