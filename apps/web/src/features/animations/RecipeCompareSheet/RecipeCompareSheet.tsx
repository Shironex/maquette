/**
 * What changed: what the library's newest revision of the library recipe a
 * project's recipe came from changes against it. Use rN copies that
 * revision into the project as a new recipe; the project's own recipe
 * stays as it is. Opened from the Motion view's Recipes tab.
 */
import { Button, InlineBanner, Loader } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { useRecipeCompareSheet } from './RecipeCompareSheet.hooks';

export default function RecipeCompareSheet() {
  const { sheet, compare, useLatest, close } = useRecipeCompareSheet();
  if (!sheet) return null;
  const changes = compare?.changes.map(line => <li key={line}>{line}</li>);
  return (
    <Sheet
      id="recipe-compare"
      icon="books"
      title="What changed"
      meta={
        compare ? `${compare.entryName}: r${compare.revision} to r${compare.latest}` : undefined
      }
      onClose={close}
      width={560}
      footer={
        <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
          <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
            Use makes a new recipe; this one stays.
          </span>
          <Button onClick={close}>Close</Button>
          <Button
            variant="primary"
            icon="plus"
            disabled={!compare || sheet.using}
            disabledReason="Wait for the library to be read"
            loading={sheet.using}
            tooltip={
              compare ? `Copies r${compare.latest} into the project as a new recipe` : undefined
            }
            onClick={useLatest}
          >
            {compare ? `Use r${compare.latest}` : 'Use'}
          </Button>
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4">
        {sheet.error ? (
          <InlineBanner tone="danger" title="Could not compare" detail={sheet.error} />
        ) : null}
        {compare ? (
          <section aria-label="What changes" className="flex flex-col gap-1.5">
            <h3 className="text-emphasis font-semibold">
              What r{compare.latest} changes against this recipe
            </h3>
            {compare.changes.length ? (
              <ul
                aria-label="Changes"
                className="m-0 flex list-disc flex-col gap-1 pl-5 text-meta text-fg-dim"
              >
                {changes}
              </ul>
            ) : (
              <p className="text-meta text-fg-mute">It holds what this recipe holds.</p>
            )}
          </section>
        ) : sheet.error ? null : (
          <Loader label="Reading the library..." />
        )}
      </div>
    </Sheet>
  );
}
