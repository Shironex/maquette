import { Button, Icon, cn } from '@maquette/ui';
import type { RecipeReviewFix } from '@/ipc/client';

export interface RecipeFixesProps {
  fixes: readonly RecipeReviewFix[];
  /** Lays a fix on the recipe in the editor; nothing is saved. */
  onTry: (fix: RecipeReviewFix) => void;
  /** Why the last Try it could not be laid on the recipe. */
  error?: string | null;
}

/**
 * Fixes, best first: for each warning a change that answers it, what it
 * should come to after the next bake and whether that passes. Try it opens
 * the recipe with the change made, unsaved, to save and bake again.
 */
export default function RecipeFixes({ fixes, onTry, error }: RecipeFixesProps) {
  if (!fixes.length) return null;
  const items = fixes.map((fix, i) => (
    <li
      key={fix.id}
      className="flex items-start gap-2 rounded-card border border-line bg-raised px-2.5 py-2"
    >
      <span className="grid size-5 flex-none place-items-center rounded-full bg-hover font-mono text-[11px] text-fg-dim">
        {i + 1}
      </span>
      <div className="min-w-0 flex-1">
        <b className="block text-meta font-semibold text-fg">{fix.title}</b>
        <p className="text-meta text-fg-mute">{fix.detail}</p>
        <p
          className={cn(
            'flex items-center gap-1 text-meta',
            fix.passes ? 'text-sage' : 'text-amber'
          )}
        >
          <Icon name={fix.passes ? 'check-circle' : 'warning'} size={13} />
          after: {fix.after}, {fix.passes ? 'passes' : 'still over'}
        </p>
      </div>
      <Button
        size="sm"
        variant="secondary"
        tooltip="Makes the change in the recipe, unsaved, to save and bake again"
        onClick={() => onTry(fix)}
      >
        Try it
      </Button>
    </li>
  ));
  return (
    <section aria-label="Fixes" className="flex flex-col gap-1.5">
      <h2 className="text-meta font-semibold text-fg-dim">Fixes, best first</h2>
      <ol className="m-0 flex list-none flex-col gap-1.5 p-0">{items}</ol>
      {error ? <p className="text-meta text-amber">{error}</p> : null}
    </section>
  );
}
