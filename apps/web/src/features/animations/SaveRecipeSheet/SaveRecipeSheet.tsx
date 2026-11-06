/**
 * Save to library: a project's recipe saved to the recipe library as the
 * next revision of the library recipe it came from, or as a new one. It
 * says what changes against the newest revision before anything is
 * written, and a save always adds a revision. Opened from the Motion
 * view's Recipes tab, and from an agent's proposal on its approval card.
 */
import { Button, InlineBanner, Loader, SegmentedControl } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { NEW_ENTRY, useSaveRecipeSheet } from './SaveRecipeSheet.hooks';

const MOVED =
  'The library recipe changed since this sheet opened. It was read again; look before saving.';

export default function SaveRecipeSheet() {
  const v = useSaveRecipeSheet();
  const { sheet, plan } = v;
  if (!sheet) return null;
  const changes = plan?.changes.map(line => <li key={line}>{line}</li>);
  const heading = !plan
    ? null
    : plan.entry === null
      ? 'What it holds'
      : plan.same
        ? `Nothing changed since r${plan.latest}.`
        : `What changes against r${plan.latest}`;
  const options = [
    ...(sheet.revisionOf
      ? [{ value: sheet.revisionOf, label: `New revision of ${v.revisionName}` }]
      : []),
    { value: NEW_ENTRY, label: 'New library recipe' },
  ];
  const saveLabel =
    sheet.entry === null ? 'Save as a new recipe' : plan ? `Save as r${plan.revision}` : 'Save';
  return (
    <Sheet
      id="recipe-save"
      icon="books"
      title="Save to library"
      meta={plan?.recipeName ?? sheet.recipe}
      onClose={v.close}
      width={560}
      footer={
        <div className="flex items-center gap-3 border-t border-line px-5 py-3.5">
          <span className="min-w-0 flex-1 text-meta text-fg-mute">
            A save adds a revision; it never overwrites one.
          </span>
          <Button onClick={v.close}>Cancel</Button>
          <Button
            variant="primary"
            icon="books"
            disabled={!plan || plan.same || sheet.planning || sheet.saving}
            disabledReason={
              plan?.same ? 'Nothing changed to save' : 'Wait for the library to be read'
            }
            loading={sheet.saving}
            onClick={v.save}
          >
            {saveLabel}
          </Button>
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4">
        {sheet.moved ? <InlineBanner tone="warning" title={MOVED} /> : null}
        {sheet.error ? <InlineBanner tone="danger" title="Not saved" detail={sheet.error} /> : null}
        <SegmentedControl
          label="Save as"
          tone="clay"
          block
          options={options}
          value={v.target}
          onChange={v.setTarget}
        />
        {sheet.planning ? (
          <Loader label="Reading the library..." />
        ) : heading ? (
          <section aria-label="What changes" className="flex flex-col gap-1.5">
            <h3 className="text-emphasis font-semibold">{heading}</h3>
            {plan?.same ? (
              <p className="text-meta text-fg-mute">
                {plan.recipeName} holds what r{plan.latest} of {plan.entryName} holds, so there is
                nothing to save.
              </p>
            ) : (
              <ul
                aria-label="Changes"
                className="m-0 flex list-disc flex-col gap-1 pl-5 text-meta text-fg-dim"
              >
                {changes}
              </ul>
            )}
            {plan && !plan.same ? (
              <p className="text-meta text-fg-mute">
                {plan.entry
                  ? `It is saved as r${plan.revision} of ${plan.entryName}; r${plan.latest} and older stay as they are.`
                  : `It becomes the library recipe ${plan.entryName}, at r1.`}
              </p>
            ) : null}
          </section>
        ) : null}
      </div>
    </Sheet>
  );
}
