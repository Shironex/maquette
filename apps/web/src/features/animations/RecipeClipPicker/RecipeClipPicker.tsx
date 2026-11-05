/**
 * New recipe: pick the clip from the animation library a recipe starts
 * from, and name it. The recipe file keeps the clip's name, frames, length
 * and hash, never its keys. Opened from the Motion view's Recipes tab.
 */
import { Button, InlineBanner, Loader, TextField } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { ClipRow } from '../ClipRow';
import { useRecipeClipPicker } from './RecipeClipPicker.hooks';

const MIXAMO_NOTE =
  'A Mixamo clip: the recipe file can be shared, but what it bakes counts as Mixamo, so export packs leave it out and it never goes into the clip library.';

export default function RecipeClipPicker() {
  const v = useRecipeClipPicker();
  if (!v.open) return null;
  const rows = v.shown.map(item => (
    <ClipRow
      key={item.clip.id}
      item={item}
      selected={v.clip?.id === item.clip.id}
      onSelect={() => v.choose(item.clip.id)}
    />
  ));
  return (
    <Sheet
      id="recipe-clip"
      icon="flask"
      title="New recipe"
      meta="Pick the base clip from the animation library"
      onClose={v.close}
      width={600}
      footer={
        <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
          <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
            {v.clip ? `On ${v.clip.name}` : 'No clip picked yet'}
          </span>
          <Button onClick={v.close}>Cancel</Button>
          <Button
            variant="primary"
            icon="flask"
            disabled={!v.clip || !v.name.trim() || v.creating}
            disabledReason={!v.clip ? 'Pick a clip first' : 'Name the recipe'}
            loading={v.creating}
            onClick={v.make}
          >
            Make recipe
          </Button>
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3 px-5 py-4">
        {v.error ? (
          <InlineBanner tone="danger" title="Could not make the recipe" detail={v.error} />
        ) : null}
        {v.libraryError ? (
          <InlineBanner
            tone="danger"
            title="The library could not be read"
            detail={v.libraryError}
          />
        ) : null}
        <TextField
          label="Search clips"
          value={v.query}
          placeholder="Name, pack or tag"
          onChange={e => v.setQuery(e.target.value)}
        />
        {v.loading ? (
          <Loader label="Reading the library..." />
        ) : v.total === 0 ? (
          <p className="text-meta text-fg-mute">
            The animation library has no clips yet. Import some in Animations first.
          </p>
        ) : (
          <ul
            aria-label="Library clips"
            className="flex min-h-0 flex-1 list-none flex-col overflow-y-auto"
          >
            {rows.length ? rows : <li className="text-meta text-fg-mute">No clip matches.</li>}
          </ul>
        )}
        {v.clip?.rig.kind === 'mixamo' ? <InlineBanner tone="info" title={MIXAMO_NOTE} /> : null}
        <TextField
          label="Recipe name"
          value={v.name}
          maxLength={120}
          onChange={e => v.setName(e.target.value)}
        />
      </div>
    </Sheet>
  );
}
