/**
 * Use in project: copies a library recipe's revision into a project as a
 * new recipe that remembers where it came from. Prop blanks the project
 * lacks open, and the notes say so. Opened from the Animations library's
 * Recipes section.
 */
import { Button, InlineBanner, Select, TextField } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import type { UseTarget } from '@/state/use-recipe-library-store';
import { useUseRecipeSheet, useUseTarget } from './UseRecipeSheet.hooks';

function UseRecipe({ target }: { target: UseTarget }) {
  const v = useUseRecipeSheet(target);
  const what = target.revision == null ? 'The starter' : `r${target.revision}`;
  const notes = v.used?.notes.map(n => <li key={n}>{n}</li>);
  const footer = v.used ? (
    <div className="flex items-center justify-end gap-3 border-t border-line px-5 py-3.5">
      <Button onClick={v.close}>Close</Button>
      <Button variant="primary" icon="flask" onClick={v.openIt}>
        Open in Motion
      </Button>
    </div>
  ) : (
    <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
      <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
        {what} into {v.projectName}
      </span>
      <Button onClick={v.close}>Cancel</Button>
      <Button
        variant="primary"
        icon="plus"
        disabled={!v.project || v.busy}
        disabledReason="Pick a project"
        loading={v.busy}
        onClick={v.use}
      >
        Use in project
      </Button>
    </div>
  );
  return (
    <Sheet
      id="use-recipe"
      icon="flask"
      title="Use in project"
      meta={
        target.revision == null ? `${target.name}, starter` : `${target.name}, r${target.revision}`
      }
      onClose={v.close}
      footer={footer}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4">
        {v.used ? (
          <InlineBanner
            tone={v.used.notes.length ? 'warning' : 'success'}
            title={`Made ${v.used.doc.recipe.name} in ${v.projectName}`}
            detail={
              v.used.notes.length ? (
                <ul aria-label="What to know" className="m-0 flex list-none flex-col gap-0.5 p-0">
                  {notes}
                </ul>
              ) : (
                "It is in the project's Recipes tab in the Motion view."
              )
            }
          />
        ) : (
          <>
            {v.error ? (
              <InlineBanner tone="danger" title="Could not use the recipe" detail={v.error} />
            ) : null}
            <Select
              label="Project"
              options={v.projects}
              value={v.project}
              onChange={v.setProject}
            />
            <TextField
              label="Recipe name"
              hint={
                target.revision == null
                  ? "Optional; the starter's name when left empty"
                  : "Optional; the library recipe's name when left empty"
              }
              placeholder={target.name}
              value={v.name}
              maxLength={120}
              onChange={e => v.setName(e.target.value)}
            />
            <p className="text-meta text-fg-mute">
              {target.revision == null
                ? 'The copy takes the first library clip with one of its names and is yours to edit. Its prop blanks start open.'
                : `The copy remembers r${target.revision}, so the Recipes tab says when the library has a newer revision. Prop blanks the project does not have open.`}
            </p>
          </>
        )}
      </div>
    </Sheet>
  );
}

export default function UseRecipeSheet() {
  const target = useUseTarget();
  if (!target) return null;
  return <UseRecipe key={`${target.entry}/${target.revision}`} target={target} />;
}
