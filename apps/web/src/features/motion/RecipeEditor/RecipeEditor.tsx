import { Button, InlineBanner } from '@maquette/ui';
import { editable } from '@/state/recipe-draft';
import { RecipeBlanks } from '../RecipeBlanks';
import { RecipeChanges } from '../RecipeChanges';
import { RecipePlayer } from '../RecipePlayer';
import { ApplyButton, EditorBake } from './EditorApply';
import { EditorDialogs, PendingEdits } from './EditorDialogs';
import RecipeReadOnly from './RecipeReadOnly';
import { useEditorApply, useRecipeEditor } from './RecipeEditor.hooks';

/**
 * One recipe open: the rough preview with its timeline, the changes and the
 * blanks. Edits stay here until Save, through leaving the tab; leaving the
 * recipe with unsaved edits asks first, and an agent's edit made meanwhile
 * merges or asks which side wins. A recipe of file version 2, which the
 * editor cannot edit yet, shows read only, with the same questions and a
 * way to save or discard edits made before an agent changed it.
 */
export default function RecipeEditor() {
  const v = useRecipeEditor();
  const applying = useEditorApply(v.saveBlocked === 'Nothing to save' ? undefined : v.saveBlocked);
  const { editing } = v;
  if (!editing) return null;
  const { doc } = editing;
  if (!editable(doc.recipe))
    return (
      <RecipeReadOnly doc={doc} onBack={v.back} pending={<PendingEdits v={v} />}>
        <EditorDialogs v={v} file={doc.file} />
      </RecipeReadOnly>
    );
  const general = v.general.map(line => <InlineBanner key={line} tone="danger" title={line} />);
  const status = v.saving ? 'saving' : v.dirty ? 'unsaved edits' : 'saved';
  return (
    <div
      aria-label={`Recipe ${doc.recipe.name}`}
      role="region"
      className="flex min-h-0 flex-1 flex-col gap-3"
    >
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="ghost" icon="arrow-left" onClick={v.back}>
          Recipes
        </Button>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-panel-title">{doc.recipe.name}</h2>
          <p className="truncate font-mono text-[11.5px] text-fg-mute">
            {doc.recipe.base.name} · {doc.recipe.base.frames} f · {doc.file} · {status}
          </p>
        </div>
        {v.outside && !v.outside.asking ? (
          <Button size="sm" icon="warning" onClick={v.outside.ask}>
            Answer clash
          </Button>
        ) : null}
        <Button size="sm" disabled={!v.dirty || v.saving} onClick={v.discard}>
          Discard
        </Button>
        <Button
          size="sm"
          variant={v.saveBlocked ? 'secondary' : 'primary'}
          icon="check"
          loading={v.saving}
          disabled={Boolean(v.saveBlocked)}
          disabledReason={v.saveBlocked}
          onClick={v.save}
        >
          Save
        </Button>
        <ApplyButton v={applying} />
      </div>
      <EditorBake v={applying} />
      {v.gone ? (
        <InlineBanner
          tone="danger"
          title="The recipe file is gone"
          detail="It was removed outside. Copy anything you need; Save cannot bring it back."
        />
      ) : null}
      {v.unreadable && !v.gone ? (
        <InlineBanner
          tone="info"
          title="The file on disk cannot be read right now"
          detail="Your edits are kept. It reads again when the window comes back or the file changes."
        />
      ) : null}
      {v.saveError ? (
        <InlineBanner tone="danger" title="The recipe was not saved" detail={v.saveError} />
      ) : null}
      {v.merged ? <InlineBanner tone="info" title={v.merged} /> : null}
      {general}
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto xl:grid-cols-[minmax(0,1.1fr)_minmax(320px,1fr)]">
        <RecipePlayer />
        <div className="flex min-w-0 flex-col gap-3">
          <RecipeChanges />
          <RecipeBlanks />
        </div>
      </div>
      <EditorDialogs v={v} file={doc.file} />
    </div>
  );
}
