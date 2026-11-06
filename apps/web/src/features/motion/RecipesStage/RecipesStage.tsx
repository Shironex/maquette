import { Button, EmptyState, Icon, IconButton, InlineBanner, Loader } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { MixamoBadge } from '@/features/common/MixamoBadge';
import type { RecipeSummary } from '@/ipc/client';
import type { LibraryNote } from '@/state/recipe-library';
import { recipeLine } from '@/state/recipes';
import { MotionTabs } from '../MotionTabs';
import { RecipeBake, useRecipeBake } from '../RecipeBake';
import { RecipeEditor } from '../RecipeEditor';
import { RecipeReview } from '../RecipeReview';
import { useRecipesStage, useRowMenu } from './RecipesStage.hooks';

const ABOUT =
  'A recipe is a clip from the animation library with changes laid over it. Apply bakes it onto a rigged model as a single clip, staged for a review.';

/** Save to library and Export, behind the row's More button. */
function RowMenu({
  name,
  onSave,
  onExport,
}: {
  name: string;
  onSave: () => void;
  onExport: () => void;
}) {
  const menu = useRowMenu();
  return (
    <span className="relative flex-none">
      <IconButton
        icon="dots-three"
        size="sm"
        label={`More for ${name}`}
        aria-haspopup="menu"
        aria-expanded={menu.open}
        onClick={menu.toggle}
      />
      {menu.open ? (
        <Menu
          label={`More for ${name}`}
          onClose={menu.close}
          className="absolute top-[calc(100%+2px)] right-0 z-30 w-[220px]"
          items={[
            {
              id: 'save',
              label: 'Save to library',
              icon: 'books',
              detail: 'As a new revision or a new library recipe',
              onSelect: onSave,
            },
            {
              id: 'export',
              label: 'Export',
              icon: 'export',
              detail: 'As a .recipe.json where you choose',
              onSelect: onExport,
            },
          ]}
        />
      ) : null}
    </span>
  );
}

/** "Library has r3" with What changed, or that the library recipe it came from is gone. */
function LibraryLine({ note, onCompare }: { note: LibraryNote | null; onCompare: () => void }) {
  if (!note) return null;
  if (note.kind === 'gone')
    return <p className="px-3 text-meta text-fg-mute">Its library recipe is gone.</p>;
  return (
    <div className="flex items-center gap-2 px-3 text-meta text-fg-dim">
      <Icon name="books" size={15} className="text-clay-text" />
      <span>Library has r{note.latest}</span>
      <Button size="sm" variant="ghost" onClick={onCompare}>
        What changed
      </Button>
    </div>
  );
}

function Row({
  project,
  recipe,
  onEdit,
  opening,
  note,
  onSave,
  onExport,
  onCompare,
}: {
  project: string;
  recipe: RecipeSummary;
  onEdit: () => void;
  opening: boolean;
  note: LibraryNote | null;
  onSave: () => void;
  onExport: () => void;
  onCompare: () => void;
}) {
  const bake = useRecipeBake(
    project,
    recipe.id,
    { open: recipe.open, layers: recipe.layers },
    recipe.motionVersion || null
  );
  if (recipe.problem) {
    return (
      <li className="flex items-start gap-3 rounded-card border border-line bg-danger-soft px-3 py-2.5">
        <Icon name="warning-circle" size={18} className="mt-0.5 flex-none text-danger" />
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{recipe.file}</b>
          <span className="block text-meta text-fg-mute">{recipe.problem}</span>
        </span>
      </li>
    );
  }
  return (
    <li className="flex flex-col gap-2">
      <div className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2.5">
        <span className="grid size-9 flex-none place-items-center rounded-[8px] bg-studio">
          <Icon name="flask" size={18} className="text-fg-dim" />
        </span>
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{recipe.name}</b>
          <span className="block truncate font-mono text-[11.5px] text-fg-mute">
            {recipeLine(recipe)}
          </span>
          {recipe.warnings.length ? (
            <span className="block text-meta text-amber">{recipe.warnings.join(' ')}</span>
          ) : null}
        </span>
        {recipe.mixamo ? <MixamoBadge /> : null}
        <Button size="sm" icon="note-pencil" loading={opening} onClick={onEdit}>
          Edit
        </Button>
        <Button
          size="sm"
          icon="play"
          loading={bake.busy && !bake.running}
          disabled={Boolean(bake.blocked)}
          disabledReason={bake.blocked ?? undefined}
          tooltip={bake.model ? `Bakes it onto ${bake.model.name}` : undefined}
          onClick={() => void bake.apply(recipe.version || null)}
        >
          Apply
        </Button>
        <RowMenu name={recipe.name} onSave={onSave} onExport={onExport} />
      </div>
      <LibraryLine note={note} onCompare={onCompare} />
      <RecipeBake
        record={bake.record}
        assetName={bake.assetName}
        busy={bake.busy}
        error={bake.error}
        stale={bake.stale}
        onCancel={bake.cancel}
        onReview={bake.review}
      />
    </li>
  );
}

/** The project's animation recipes, with a new one made from a library clip. */
export default function RecipesStage() {
  const {
    project,
    recipes,
    error,
    retry,
    newRecipe,
    editing,
    reviewing,
    edit,
    opening,
    openError,
    ...library
  } = useRecipesStage();
  const rows = recipes?.map(r => (
    <Row
      key={r.id}
      project={project ?? ''}
      recipe={r}
      opening={opening === r.id}
      onEdit={() => edit(r.id)}
      note={library.noteOf(r)}
      onSave={() => library.saveToLibrary(r.id)}
      onExport={() => library.exportFile(r)}
      onCompare={() => library.compare(r.id)}
    />
  ));
  const importButton = (
    <Button
      icon="download-simple"
      loading={library.importing}
      tooltip="Brings a .recipe.json in as a new recipe"
      onClick={library.importFile}
    >
      Import
    </Button>
  );
  const notes = library.imported?.notes.map(n => <li key={n}>{n}</li>);
  const importNote = library.imported ? (
    <InlineBanner
      tone={!library.imported.name ? 'danger' : notes?.length ? 'warning' : 'success'}
      title={
        library.imported.name ? `Imported ${library.imported.name}` : 'The recipe was not imported'
      }
      detail={
        notes?.length ? (
          <ul aria-label="Import notes" className="m-0 flex list-none flex-col gap-0.5 p-0">
            {notes}
          </ul>
        ) : undefined
      }
      actions={<IconButton icon="x" size="sm" label="Dismiss" onClick={library.dismissImported} />}
    />
  ) : null;
  const add = (
    <Button
      icon="plus"
      tooltip="Starts a recipe from a clip in the animation library"
      onClick={newRecipe}
    >
      New recipe
    </Button>
  );
  const body = error ? (
    <InlineBanner
      tone="danger"
      title="The recipes could not be read"
      detail={error}
      actions={
        <Button size="sm" icon="arrow-clockwise" onClick={retry}>
          Try again
        </Button>
      }
    />
  ) : recipes === null ? (
    <Loader label="Reading the recipes..." />
  ) : recipes.length === 0 ? (
    <div className="grid flex-1 place-items-center">
      <EmptyState
        icon="flask"
        title="No recipes yet"
        body="Start one from a clip in the animation library, then lay changes over it: reach, nudge, look at and wave. Apply bakes it onto a rigged model."
        action={add}
      />
    </div>
  ) : (
    <ul aria-label="Project recipes" className="flex list-none flex-col gap-2 overflow-y-auto">
      {rows}
    </ul>
  );
  return (
    <section aria-label="Recipes" className="flex min-w-0 flex-1 flex-col gap-3 px-5 py-4">
      <MotionTabs />
      {editing ? (
        <RecipeEditor />
      ) : reviewing ? (
        <RecipeReview />
      ) : (
        <>
          <div className="flex items-start gap-3">
            <div className="min-w-0 flex-1">
              <h1 className="text-panel-title">Recipes</h1>
              <p className="text-meta text-fg-mute">{ABOUT}</p>
            </div>
            {importButton}
            {recipes?.length ? add : null}
          </div>
          {importNote}
          {openError ? (
            <InlineBanner tone="danger" title="The recipe could not be opened" detail={openError} />
          ) : null}
          {body}
        </>
      )}
    </section>
  );
}
