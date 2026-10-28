import { TopBar, cn } from '@maquette/ui';
import { contentKey } from '@/state/inbox';
import { InboxDetail } from '../InboxDetail';
import { InboxEmpty } from '../InboxEmpty';
import { InboxRow } from '../InboxRow';
import { RedoBar } from '../RedoBar';
import { useInboxRows, useInboxView } from './InboxView.hooks';

/** The models waiting in the inbox folder, and the one on show with its checks. */
export default function InboxView() {
  const {
    snapshot,
    items,
    item,
    select,
    checked,
    check,
    clearChecked,
    menuFor,
    toggleMenu,
    closeMenu,
    redoBlocker,
    redoChecked,
  } = useInboxView();
  const { scrollRef, listRef, shown } = useInboxRows(items.length);
  const rows = items.slice(shown.start, shown.end).map((i, n) => (
    <li key={i.id} {...shown.itemProps(shown.start + n)}>
      <InboxRow
        item={i}
        selected={i.id === item?.id}
        onSelect={() => select(i.id)}
        checked={checked.includes(i.id)}
        selecting={checked.length > 0}
        onCheck={on => check(i.id, on)}
        menuOpen={menuFor === i.id}
        onMenu={() => toggleMenu(i.id)}
        onCloseMenu={closeMenu}
      />
    </li>
  ));

  return (
    <>
      <TopBar lead={<h1 className="text-panel-title">Inbox</h1>} />
      <main className="flex min-h-0 flex-1">
        <section
          aria-label="Models in the inbox"
          className="flex w-[300px] flex-none flex-col border-r border-line bg-panel"
        >
          <div className="px-4 pt-[18px] pb-3">
            <h2 className="text-panel-title">Model inbox</h2>
            <p className="mt-0.5 text-fg-mute">
              {items.length === 0
                ? 'Empty'
                : `${items.length} ${items.length === 1 ? 'model' : 'models'} waiting`}
            </p>
          </div>
          <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
            <ul
              ref={listRef}
              style={{ paddingTop: shown.before, paddingBottom: shown.after }}
              className="flex list-none flex-col gap-1"
            >
              {rows}
            </ul>
          </div>
          {checked.length > 0 ? (
            <RedoBar
              count={checked.length}
              blocker={redoBlocker}
              onRedo={redoChecked}
              onClear={clearChecked}
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
        </section>
        {/* A finished download or a new file under the same name starts the preview over. */}
        {item ? (
          <InboxDetail key={contentKey(item)} item={item} />
        ) : (
          <InboxEmpty dir={snapshot?.dir} />
        )}
      </main>
    </>
  );
}
