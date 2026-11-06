/**
 * The starter recipes under the library recipes: built into the app and
 * read only, each showing a kind of change. Selecting one opens it beside
 * the list, where it can be used in a project or saved to the library.
 */
import { Icon, cn } from '@maquette/ui';
import type { RecipeStarter } from '@/ipc/client';
import { starterLine } from '@/state/recipe-starters';

export interface StarterRecipesProps {
  starters: readonly RecipeStarter[];
  selected: string | null;
  onSelect: (id: string) => void;
}

function Row({
  starter,
  selected,
  onSelect,
}: {
  starter: RecipeStarter;
  selected: boolean;
  onSelect: () => void;
}) {
  const missing = !starter.rest && starter.base.status === 'missing';
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={`${starter.name}, starter`}
        onClick={onSelect}
        className={cn(
          'flex w-full cursor-pointer items-center gap-3 rounded-card border border-dashed border-line bg-raised px-3 py-2.5 text-left hover:bg-hover',
          selected && 'border-clay/40 bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <span className="grid size-9 flex-none place-items-center rounded-[8px] bg-studio">
          <Icon name="sparkle" size={18} className="text-fg-dim" />
        </span>
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{starter.name}</b>
          <span className="block truncate font-mono text-[11.5px] text-fg-mute">
            {starterLine(starter)}
          </span>
        </span>
        {missing ? (
          <span
            title="No clip of that name in the clip library yet"
            className="flex-none text-[11.5px] text-amber"
          >
            Needs clip
          </span>
        ) : null}
        <span className="flex-none rounded-pill border border-line px-2 text-[11.5px] leading-5 text-fg-dim">
          Starter
        </span>
      </button>
    </li>
  );
}

export default function StarterRecipes({ starters, selected, onSelect }: StarterRecipesProps) {
  const rows = starters.map(s => (
    <Row key={s.id} starter={s} selected={s.id === selected} onSelect={() => onSelect(s.id)} />
  ));
  return (
    <section aria-label="Starters" className="flex flex-col gap-2">
      <div className="flex items-baseline gap-3">
        <h3 className="text-caption tracking-[0.04em] text-fg-mute uppercase">Starters</h3>
        <span className="text-meta text-fg-mute">Built in, read only. Save one to edit it.</span>
      </div>
      <ul aria-label="Starter recipes" className="flex list-none flex-col gap-2">
        {rows}
      </ul>
    </section>
  );
}
