/**
 * The Animations library's Recipes: recipes kept once outside any project,
 * each with its revisions, saved back from a project's Recipes tab or
 * imported from a `.recipe.json`, and under them the starters built into
 * the app. The selected one opens beside the list.
 */
import { Button, EmptyState, Icon, IconButton, InlineBanner, Loader, cn } from '@maquette/ui';
import { MixamoBadge } from '@/features/common/MixamoBadge';
import type { LibraryRecipeSummary } from '@/ipc/client';
import { counted } from '@/lib/format';
import { libraryLine } from '@/state/recipe-library';
import { LibraryRecipeDetail } from '../LibraryRecipeDetail';
import { StarterDetail } from '../StarterDetail';
import { StarterRecipes } from '../StarterRecipes';
import { useLibraryRecipes } from './LibraryRecipes.hooks';

function Row({
  entry,
  selected,
  onSelect,
}: {
  entry: LibraryRecipeSummary;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={entry.name}
        onClick={onSelect}
        className={cn(
          'flex w-full cursor-pointer items-center gap-3 rounded-card border border-line bg-raised px-3 py-2.5 text-left hover:bg-hover',
          selected && 'border-clay/40 bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <span className="grid size-9 flex-none place-items-center rounded-[8px] bg-studio">
          <Icon
            name={entry.problem ? 'warning-circle' : 'flask'}
            size={18}
            className={entry.problem ? 'text-danger' : 'text-fg-dim'}
          />
        </span>
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{entry.name}</b>
          {entry.problem ? (
            <span className="block text-meta text-danger">{entry.problem}</span>
          ) : (
            <span className="block truncate font-mono text-[11.5px] text-fg-mute">
              {libraryLine(entry)}
            </span>
          )}
        </span>
        {entry.mixamo ? <MixamoBadge /> : null}
      </button>
    </li>
  );
}

export default function LibraryRecipes() {
  const v = useLibraryRecipes();
  const importButton = (primary: boolean) => (
    <Button
      variant={primary ? 'primary' : undefined}
      icon="download-simple"
      loading={v.importing}
      tooltip="Adds a .recipe.json as a new library recipe at r1"
      onClick={v.importFile}
    >
      Import recipe
    </Button>
  );
  const rows = v.list?.map(entry => (
    <Row
      key={entry.id}
      entry={entry}
      selected={entry.id === v.summary?.id}
      onSelect={() => v.select(entry.id)}
    />
  ));
  const noteItems = v.imported?.notes.map(n => <li key={n}>{n}</li>);
  const note = v.imported ? (
    <InlineBanner
      tone={v.imported.notes.length ? 'warning' : 'success'}
      title={
        v.imported.starter
          ? `Saved ${v.imported.name} to the library as r1`
          : `Imported ${v.imported.name} as r1`
      }
      detail={
        v.imported.notes.length ? (
          <ul className="m-0 flex list-none flex-col gap-0.5 p-0">{noteItems}</ul>
        ) : undefined
      }
      actions={<IconButton icon="x" size="sm" label="Dismiss" onClick={v.dismissImported} />}
    />
  ) : null;
  const body =
    v.list === null ? (
      v.error ? null : (
        <div className="grid flex-1 place-items-center">
          <Loader label="Reading the recipes..." />
        </div>
      )
    ) : v.list.length === 0 ? (
      <EmptyState
        icon="flask"
        title="No library recipes yet"
        body="Save a recipe from the Motion view's Recipes tab, import a .recipe.json, or start from a starter below."
        action={importButton(true)}
      />
    ) : (
      <ul aria-label="Library recipes" className="flex list-none flex-col gap-2">
        {rows}
      </ul>
    );
  const starters = v.starters?.length ? (
    <StarterRecipes starters={v.starters} selected={v.starter?.id ?? null} onSelect={v.select} />
  ) : null;
  return (
    <main className="flex min-h-0 flex-1" aria-busy={v.list === null && !v.error}>
      <section
        aria-label="Recipes"
        className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
      >
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <h2 className="text-panel-title whitespace-nowrap">Library recipes</h2>
          {v.list ? (
            <span className="text-meta text-fg-mute">{counted(v.list.length, 'recipe')}</span>
          ) : null}
          <span className="flex-1" />
          {v.list?.length ? importButton(false) : null}
        </div>
        {note}
        {v.importError ? (
          <InlineBanner tone="danger" title="The recipe was not imported" detail={v.importError} />
        ) : null}
        {v.error ? (
          <InlineBanner
            tone="danger"
            title="The recipe library could not be read"
            detail={v.error}
            actions={
              <Button size="sm" icon="arrow-clockwise" onClick={v.retry}>
                Try again
              </Button>
            }
          />
        ) : null}
        {body}
        {starters}
      </section>
      {v.summary ? <LibraryRecipeDetail key={v.summary.id} summary={v.summary} /> : null}
      {v.starter ? <StarterDetail key={v.starter.id} starter={v.starter} /> : null}
    </main>
  );
}
