import { Icon, cn } from '@maquette/ui';
import type { LibraryGripSummary } from '@/ipc/client';
import { WEAPON_GROUPS } from '@/state/recipe-grips';
import { gripLine } from '@/state/recipe-library';
import { filterGrips, type GripFilter } from '@/state/use-grip-library-store';
import { GripPicture } from '../GripPicture';

export interface GripGridProps {
  list: readonly LibraryGripSummary[];
  filter: GripFilter;
  onFilter: (filter: GripFilter) => void;
  selected: string | null;
  onSelect: (entry: string) => void;
  /** Names the grid for assistive tech: "Library grips". */
  label: string;
}

/** The filters the grid offers, All first; Other has no button of its own. */
const FILTERS: readonly (readonly [GripFilter, string])[] = [
  ['all', 'All'],
  ...(WEAPON_GROUPS.filter(([id]) => id !== 'other') as [GripFilter, string][]),
];

function Card({
  grip,
  selected,
  onSelect,
}: {
  grip: LibraryGripSummary;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={grip.name}
        onClick={onSelect}
        className={cn(
          'flex w-full cursor-pointer flex-col overflow-hidden rounded-card border border-line bg-raised text-left hover:bg-hover',
          selected && 'border-clay/50 bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <span className="grid h-[92px] place-items-center bg-studio px-3 py-2">
          {grip.problem ? (
            <Icon name="warning-circle" size={22} className="text-danger" />
          ) : (
            <GripPicture
              pose={grip.pose}
              holds={grip.hands?.holds}
              label={`${grip.name}, ${grip.handsWords}`}
            />
          )}
        </span>
        <span className="flex flex-col gap-0.5 px-2.5 py-2">
          <b className="truncate text-[13px] font-semibold">{grip.name}</b>
          <span
            className={cn(
              'truncate font-mono text-[11px]',
              grip.problem ? 'text-danger' : 'text-fg-mute'
            )}
          >
            {grip.problem ?? gripLine(grip)}
          </span>
        </span>
      </button>
    </li>
  );
}

/**
 * Library grips as a grid of cards, each a schematic hand drawn from its
 * newest pose with its name and line, under the weapon filters. Shared by
 * the Animations library's Grips and the recipe editor's grip picker.
 */
export default function GripGrid({
  list,
  filter,
  onFilter,
  selected,
  onSelect,
  label,
}: GripGridProps) {
  const shown = filterGrips(list, filter);
  const filters = FILTERS.map(([id, word]) => (
    <button
      key={id}
      type="button"
      aria-pressed={filter === id}
      onClick={() => onFilter(id)}
      className={cn(
        'h-6 cursor-pointer rounded-full px-2.5 text-[12px] font-medium text-fg-dim hover:bg-hover',
        filter === id && 'bg-clay-soft text-clay-text hover:bg-clay-soft'
      )}
    >
      {word}
    </button>
  ));
  const cards = shown.map(grip => (
    <Card
      key={grip.id}
      grip={grip}
      selected={grip.id === selected}
      onSelect={() => onSelect(grip.id)}
    />
  ));
  return (
    <div className="flex flex-col gap-3">
      <div role="group" aria-label="Weapon" className="flex flex-wrap gap-1.5">
        {filters}
      </div>
      {shown.length ? (
        <ul
          aria-label={label}
          className="grid list-none grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2.5"
        >
          {cards}
        </ul>
      ) : (
        <p className="py-6 text-center text-meta text-fg-mute">
          No grip for this weapon yet. Pick All.
        </p>
      )}
    </div>
  );
}
