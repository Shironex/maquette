import { Button, EmptyState, Icon, InlineBanner, Loader } from '@maquette/ui';
import type { RecipeSummary } from '@/ipc/client';
import { APPLY_LATER, recipeLine } from '@/state/recipes';
import { MotionTabs } from '../MotionTabs';
import { useRecipesStage } from './RecipesStage.hooks';

const ABOUT =
  'A recipe is a clip from the animation library with changes laid over it. Applying one bakes a single clip into the next version of a model.';

function Row({ recipe }: { recipe: RecipeSummary }) {
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
    <li className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2.5">
      <span className="grid size-9 flex-none place-items-center rounded-[8px] bg-studio">
        <Icon name="flask" size={18} className="text-fg-dim" />
      </span>
      <span className="min-w-0 flex-1">
        <b className="block truncate font-semibold">{recipe.name}</b>
        <span className="block truncate font-mono text-[11.5px] text-fg-mute">
          {recipeLine(recipe)}
        </span>
      </span>
      {recipe.mixamo ? (
        <span
          title="Its bake counts as Mixamo's: kept out of export packs. The recipe file can still be shared."
          className="flex-none rounded-pill border border-line px-2 text-[11.5px] leading-5 text-fg-dim"
        >
          Mixamo
        </span>
      ) : null}
      <Button size="sm" icon="play" disabled disabledReason={APPLY_LATER}>
        Apply
      </Button>
    </li>
  );
}

/** The project's animation recipes, with a new one made from a library clip. */
export default function RecipesStage() {
  const { recipes, error, retry, newRecipe } = useRecipesStage();
  const rows = recipes?.map(r => <Row key={r.id} recipe={r} />);
  const add = (
    <Button
      icon="plus"
      tooltip="Starts a recipe from a clip in the animation library"
      onClick={newRecipe}
    >
      New recipe
    </Button>
  );
  return (
    <section aria-label="Recipes" className="flex min-w-0 flex-1 flex-col gap-3 px-5 py-4">
      <MotionTabs />
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <h1 className="text-panel-title">Recipes</h1>
          <p className="text-meta text-fg-mute">{ABOUT}</p>
        </div>
        {recipes?.length ? add : null}
      </div>
      {error ? (
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
            body="Start one from a clip in the animation library. Its changes (reach, nudge, look at and wave) and baking come in later updates."
            action={add}
          />
        </div>
      ) : (
        <ul aria-label="Project recipes" className="flex list-none flex-col gap-2 overflow-y-auto">
          {rows}
        </ul>
      )}
    </section>
  );
}
