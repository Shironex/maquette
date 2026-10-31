import {
  Button,
  EmptyState,
  SegmentedControl,
  TextField,
  TopBar,
  type WindowItemProps,
} from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { groupItems } from '@/state/library';
import { AddManySheet } from '../AddManySheet';
import { LibraryBulkBar } from '../LibraryBulkBar';
import { LibraryCard } from '../LibraryCard';
import { LibraryDetail } from '../LibraryDetail';
import { LibraryFilters } from '../LibraryFilters';
import { LibraryRow } from '../LibraryRow';
import { LibraryStats } from '../LibraryStats';
import { LibrarySummary } from '../LibrarySummary';
import { LookAlikesView } from '../LookAlikesView';
import { PackSheet } from '../PackSheet';
import LibraryGroups from './LibraryGroups';
import { useLibraryView, useModelSelection, useModelsWindow } from './LibraryView.hooks';

const EMPTY_STEPS = [
  { title: 'Add a model', detail: 'Add to library... on a Board card or in the Inspector' },
  { title: 'Use it in a project', detail: 'The model, its picture and fix-ups copy in' },
  { title: 'Update when it changes', detail: 'Projects show when the library has a newer version' },
];

/**
 * The workspace's model library: every model a project added, in
 * collections and tags, the projects that use each, Use in project, and
 * bulk actions over a selection. Models copy into a project; nothing here
 * changes a project until you ask.
 */
export default function LibraryView() {
  const v = useLibraryView();
  const { filters, setFilters, layout, setLayout, sort, setSort, item, select } = v;
  const groups = groupItems(v.shown, v.group);
  // Shift picks a range in the order the models are on screen.
  const order = groups.flatMap(g => g.items.map(i => i.entry.id));
  const pick = useModelSelection(order, select);
  const picked = v.shown.filter(i => pick.isPicked(i.entry.id));
  const grouped = v.group !== 'none';
  const { scrollRef, listRef, shown } = useModelsWindow(grouped ? 0 : v.shown.length, layout);
  const pad = { paddingTop: shown.before, paddingBottom: shown.after };
  const build = (i: LibraryItem, position?: WindowItemProps) => {
    const id = i.entry.id;
    const selection = {
      checked: pick.isPicked(id),
      selecting: pick.selecting,
      onCheck: pick.check(id),
    };
    return layout === 'list' ? (
      <LibraryRow
        key={id}
        position={position}
        item={i}
        selected={!pick.selecting && id === item?.entry.id}
        onSelect={pick.click(id)}
        {...selection}
      />
    ) : (
      <li key={id} {...position}>
        <LibraryCard
          item={i}
          selected={!pick.selecting && id === item?.entry.id}
          onSelect={pick.click(id)}
          onInspect={v.quickView(i)}
          flag={v.flags.get(id) ?? null}
          {...selection}
        />
      </li>
    );
  };
  const cards = v.shown
    .slice(shown.start, shown.end)
    .map((i, n) => build(i, shown.itemProps(shown.start + n)));
  const addOne = (
    <Button
      icon="plus"
      tooltip="Pick a model from a project to keep in the library"
      onClick={() => v.openAdd('pick')}
    >
      Add from a project
    </Button>
  );

  return (
    <>
      <TopBar
        lead={
          <div className="flex items-baseline gap-2.5">
            <h1 className="text-panel-title">Models</h1>
            {v.loaded ? (
              <span className="text-meta text-fg-mute">{v.items.length} in the library</span>
            ) : null}
          </div>
        }
        right={
          <div className="flex items-center gap-2">
            <TextField
              type="search"
              icon="magnifying-glass"
              aria-label="Find a model"
              placeholder="Name, tag, project, by:tripo, <5k"
              value={filters.query}
              maxLength={120}
              onChange={e => setFilters({ query: e.target.value })}
              fieldClassName="w-[260px]"
            />
            <SegmentedControl
              label="Layout"
              options={[
                { value: 'grid', label: 'Grid', icon: 'squares-four' },
                { value: 'list', label: 'List', icon: 'list' },
                { value: 'look-alikes', label: 'Look-alikes', icon: 'copy' },
              ]}
              value={layout}
              onChange={setLayout}
            />
            {addOne}
            <Button
              icon="package"
              disabled={v.shown.length === 0}
              disabledReason="No model in view"
              tooltip={v.pack.tip}
              onClick={v.pack.start}
            >
              Export pack
            </Button>
            <Button
              icon="books"
              tooltip="Adds every accepted model of a project at once, names and tags set first"
              onClick={v.openAddMany}
            >
              Add a whole project
            </Button>
          </div>
        }
      />
      {v.loaded && v.items.length === 0 ? (
        <main className="flex min-h-0 flex-1 items-center justify-center p-6">
          <EmptyState
            icon="books"
            title="The library is empty"
            body="Add a model from a project and any project in this workspace can use a copy of it, without spending credits again."
            steps={EMPTY_STEPS}
            action={addOne}
          />
        </main>
      ) : (
        <main className="flex min-h-0 flex-1" aria-busy={!v.loaded}>
          <LibraryFilters
            items={v.items}
            filters={filters}
            onChange={setFilters}
            collections={v.collections}
            selected={pick.selected}
          />
          {layout === 'look-alikes' ? (
            <LookAlikesView report={v.report} items={v.items} />
          ) : (
            <>
              <div className="flex min-w-0 flex-1 flex-col">
                {picked.length ? (
                  <div className="px-6 pt-3">
                    <LibraryBulkBar
                      picked={picked}
                      shown={v.shown.length}
                      onSelectAll={pick.selectAll}
                      onClear={pick.clear}
                      known={v.known}
                      collections={v.collections}
                    />
                  </div>
                ) : null}
                <section
                  ref={scrollRef}
                  aria-label="Models"
                  className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
                >
                  <LibraryStats stats={v.stats} />
                  <LibrarySummary
                    title={v.title}
                    count={v.count}
                    pairs={v.report?.pairs.length ?? null}
                    untagged={v.untagged}
                    sort={sort}
                    onSort={setSort}
                    group={v.group}
                    onGroup={v.setGroup}
                    onLookAlikes={() => setLayout('look-alikes')}
                    onUntagged={v.showUntagged}
                  />
                  {v.error ? (
                    <p role="alert" className="text-meta text-danger">
                      {v.error}
                    </p>
                  ) : null}
                  {v.shown.length === 0 && v.loaded ? (
                    <p className="py-10 text-center text-fg-mute">
                      No model matches. Clear the search or pick All models.
                    </p>
                  ) : grouped ? (
                    <LibraryGroups
                      groups={groups}
                      list={layout === 'list'}
                      render={i => build(i)}
                    />
                  ) : layout === 'list' ? (
                    <ul ref={listRef} style={pad} className="flex list-none flex-col gap-1">
                      {cards}
                    </ul>
                  ) : (
                    <ul
                      ref={listRef}
                      style={pad}
                      className="grid list-none grid-cols-[repeat(auto-fill,minmax(172px,1fr))] gap-3"
                    >
                      {cards}
                    </ul>
                  )}
                </section>
              </div>
              {item && !pick.selecting ? <LibraryDetail key={item.entry.id} item={item} /> : null}
            </>
          )}
        </main>
      )}
      {v.addManyOpen ? <AddManySheet /> : null}
      {v.pack.open ? <PackSheet /> : null}
    </>
  );
}
