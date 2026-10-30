import { cn } from '@maquette/ui';
import { BulkDiscardDialog } from '../BulkDiscardDialog';
import { InboxRow } from '../InboxRow';
import { SelectionBar } from '../SelectionBar';
import { useSelection } from '../SelectionBar/SelectionBar.hooks';
import { TriageChips } from '../TriageChips';
import { useInboxList } from './InboxList.hooks';

export interface InboxListProps {
  /** The model on show in the detail. */
  shownId: string | null;
}

/** The inbox column: the filter chips, the models, the bar for the ticked ones and the folder. */
export default function InboxList({ shownId }: InboxListProps) {
  const list = useInboxList(shownId);
  const sel = useSelection();
  const { snapshot, all, items, shown } = list;
  const rows = items.slice(shown.start, shown.end).map((i, n) => (
    <li key={i.id} {...shown.itemProps(shown.start + n)}>
      <InboxRow
        item={i}
        selected={i.id === shownId}
        onSelect={() => list.select(i.id)}
        checked={sel.checked.includes(i.id)}
        selecting={sel.checked.length > 0}
        onCheck={on => sel.check(i.id, on)}
        menuOpen={list.menuFor === i.id}
        onMenu={() => list.toggleMenu(i.id)}
        onCloseMenu={list.closeMenu}
      />
    </li>
  ));
  const waiting = `${all.length} ${all.length === 1 ? 'model' : 'models'} waiting`;
  return (
    <section
      aria-label="Models in the inbox"
      className="flex w-[300px] flex-none flex-col border-r border-line bg-panel"
    >
      <div className="flex flex-col gap-2.5 px-4 pt-[18px] pb-3">
        <div>
          <h2 className="text-panel-title">Model inbox</h2>
          <p className="mt-0.5 text-fg-mute">
            {all.length === 0
              ? 'Empty'
              : items.length === all.length
                ? waiting
                : `${items.length} of ${waiting}`}
          </p>
        </div>
        {all.length > 0 ? (
          <TriageChips items={all} value={list.filter} onChange={list.setFilter} />
        ) : null}
      </div>
      <div ref={list.scrollRef} className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        {items.length === 0 && all.length > 0 ? (
          <p className="px-2 py-3 text-fg-mute">No model fits this filter.</p>
        ) : null}
        <ul
          ref={list.listRef}
          style={{ paddingTop: shown.before, paddingBottom: shown.after }}
          className="flex list-none flex-col gap-1"
        >
          {rows}
        </ul>
      </div>
      {sel.checked.length > 0 ? (
        <SelectionBar
          count={sel.checked.length}
          note={sel.note}
          acceptable={sel.plan.ids.length}
          redoBlocker={sel.redoBlocker}
          working={sel.working}
          onAccept={() => sel.accept()}
          onRedo={sel.redoChecked}
          onDiscard={() => sel.askDiscard()}
          onClear={sel.clearChecked}
        />
      ) : null}
      <div className="flex items-center gap-2 border-t border-line px-4 py-3">
        <span
          className={cn('size-2 rounded-full', snapshot?.watching ? 'bg-sage' : 'bg-fg-mute')}
        />
        <span className="flex-1 truncate font-mono text-[12px] text-fg-mute">
          {snapshot?.dir ?? 'inbox'}
        </span>
        <span className="text-meta text-fg-mute">
          {snapshot?.watching ? 'watching' : 'not watched'}
        </span>
      </div>
      {sel.discarding ? (
        <BulkDiscardDialog
          items={all.filter(i => sel.discarding?.includes(i.id))}
          busy={sel.working}
          onConfirm={() => void sel.confirmDiscard()}
          onClose={sel.closeDiscard}
        />
      ) : null}
    </section>
  );
}
