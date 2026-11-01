import { Button, Loader, SegmentedControl, TopBar, type SegmentedOption } from '@maquette/ui';
import { contentKey } from '@/state/inbox';
import type { InboxLayout } from '@/state/use-triage-store';
import { BlenderBanner } from '../BlenderBanner';
import { BulkDiscardDialog } from '../BulkDiscardDialog';
import { InboxClear } from '../InboxClear';
import { InboxDetail } from '../InboxDetail';
import { InboxEmpty } from '../InboxEmpty';
import { InboxList } from '../InboxList';
import { RigDoctor, RigOpening } from '../RigDoctor';
import { SelectionBar } from '../SelectionBar';
import { useSelection } from '../SelectionBar/SelectionBar.hooks';
import { TriageAside } from '../TriageAside';
import { TriageTable } from '../TriageTable';
import { useInboxView, useRigCases } from './InboxView.hooks';

const LAYOUTS: readonly SegmentedOption<InboxLayout>[] = [
  {
    value: 'one',
    label: 'One by one',
    icon: 'cube',
    hint: 'Each model with its preview and checks',
  },
  { value: 'table', label: 'Table', icon: 'list', hint: 'Every model grouped by what it needs' },
];

/** Every model in one table with the bulk bar and what the selection does. */
function TableLayout({ replaced }: { replaced: ReturnType<typeof useInboxView>['replaced'] }) {
  const sel = useSelection();
  const n = sel.items.length;
  return (
    <>
      <section
        aria-label="Models in the inbox"
        className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 pt-[18px] pb-6"
      >
        <div>
          <h2 className="text-panel-title">Model inbox</h2>
          <p className="mt-0.5 text-fg-mute">
            {n} {n === 1 ? 'model' : 'models'}, grouped by what they need, worst last.
          </p>
        </div>
        {sel.checked.length > 0 ? (
          <SelectionBar
            wide
            count={sel.checked.length}
            note={sel.note}
            acceptable={sel.plan.ids.length}
            redoBlocker={sel.redoBlocker}
            working={sel.working}
            onAccept={() => sel.accept()}
            onAcceptToLibrary={() => sel.accept({ library: true, tags: [] })}
            onRedo={sel.redoChecked}
            onDiscard={() => sel.askDiscard()}
            onClear={sel.clearChecked}
          />
        ) : null}
        <TriageTable
          items={sel.items}
          checked={sel.checked}
          onCheck={sel.check}
          onCheckAll={sel.checkAll}
        />
      </section>
      <TriageAside
        count={sel.checked.length}
        plan={sel.plan}
        replaced={replaced}
        working={sel.working}
        onAccept={after => sel.accept(after)}
        onDiscard={ids => sel.askDiscard(ids)}
      />
      {sel.discarding ? (
        <BulkDiscardDialog
          items={sel.items.filter(i => sel.discarding?.includes(i.id))}
          busy={sel.working}
          onConfirm={() => void sel.confirmDiscard()}
          onClose={sel.closeDiscard}
        />
      ) : null}
    </>
  );
}

/** The models waiting in the inbox folder: one by one with its checks, or all in one table. */
export default function InboxView() {
  const { snapshot, reading, estimate, items, item, layout, setLayout, replaced } = useInboxView();
  const empty = snapshot !== null && items.length === 0;
  const { rigCase, rigOpening, going, goingLabel, showCase } = useRigCases();
  if (rigOpening) return <RigOpening name={rigOpening.name} />;
  if (rigCase) return <RigDoctor record={rigCase} />;
  return (
    <>
      <TopBar
        lead={<h1 className="text-panel-title">Inbox</h1>}
        right={
          <div className="flex items-center gap-2">
            {going ? (
              <Button
                size="sm"
                variant="secondary"
                icon="wrench"
                tooltip="Back to this rig doctor case"
                onClick={() => void showCase(going.id)}
              >
                {goingLabel}
              </Button>
            ) : null}
            {empty ? null : (
              <SegmentedControl
                label="Inbox layout"
                tone="neutral"
                options={LAYOUTS}
                value={layout}
                onChange={setLayout}
              />
            )}
          </div>
        }
      />
      <BlenderBanner />
      <main className="flex min-h-0 flex-1">
        {reading ? (
          <div className="grid flex-1 place-items-center">
            <Loader label="Reading the inbox..." estimate={estimate} />
          </div>
        ) : empty ? (
          <InboxClear dir={snapshot.dir} watching={snapshot.watching} />
        ) : layout === 'table' ? (
          <TableLayout replaced={replaced} />
        ) : (
          <>
            <InboxList shownId={item?.id ?? null} />
            {/* A finished download or a new file under the same name starts the preview over. */}
            {item ? (
              <InboxDetail key={contentKey(item)} item={item} />
            ) : (
              <InboxEmpty dir={snapshot?.dir} />
            )}
          </>
        )}
      </main>
    </>
  );
}
