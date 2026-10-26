import { Button, EmptyState, SegmentedControl, TextField, TopBar } from '@maquette/ui';
import type { LibrarySort } from '@/state/library';
import { LibraryCard } from '../LibraryCard';
import { LibraryDetail } from '../LibraryDetail';
import { LibraryFilters } from '../LibraryFilters';
import { LibraryRow } from '../LibraryRow';
import { LibraryStats } from '../LibraryStats';
import { useLibraryView } from './LibraryView.hooks';

const SORTS: readonly { value: LibrarySort; label: string }[] = [
  { value: 'recent', label: 'Recent' },
  { value: 'used', label: 'Most used' },
  { value: 'name', label: 'Name' },
];

const SORT_NOTE: Record<LibrarySort, string> = {
  recent: 'newest first',
  used: 'most used first',
  name: 'by name',
};

/**
 * The workspace's model library: every model a project added, the projects
 * that use each, and Use in project. Models copy into a project; nothing
 * here changes a project until you ask.
 */
export default function LibraryView() {
  const v = useLibraryView();
  const { filters, setFilters, layout, setLayout, sort, setSort, item, select } = v;
  const count = (n: number) => `${n} ${n === 1 ? 'model' : 'models'}`;
  const filtered = v.shown.length !== v.items.length;
  const cards = v.shown.map(i =>
    layout === 'grid' ? (
      <li key={i.entry.id}>
        <LibraryCard
          item={i}
          selected={i.entry.id === item?.entry.id}
          onSelect={() => select(i.entry.id)}
        />
      </li>
    ) : (
      <LibraryRow
        key={i.entry.id}
        item={i}
        selected={i.entry.id === item?.entry.id}
        onSelect={() => select(i.entry.id)}
      />
    )
  );

  return (
    <>
      <TopBar
        lead={<h1 className="text-panel-title">Library</h1>}
        right={
          <div className="flex items-center gap-2">
            <TextField
              type="search"
              icon="magnifying-glass"
              aria-label="Find a model"
              placeholder="Find a model, tag or project"
              value={filters.query}
              maxLength={120}
              onChange={e => setFilters({ query: e.target.value })}
              fieldClassName="w-[280px]"
            />
            <SegmentedControl
              label="Layout"
              options={[
                { value: 'grid', label: 'Grid', icon: 'squares-four' },
                { value: 'list', label: 'List', icon: 'list' },
              ]}
              value={layout}
              onChange={setLayout}
            />
            <Button icon="plus" onClick={() => v.openAdd('pick')}>
              Add from a project
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
            steps={[
              {
                title: 'Add a model',
                detail: 'Add to library... on a Board card or in the Inspector',
              },
              {
                title: 'Use it in a project',
                detail: 'The model, its picture and fix-ups copy in',
              },
              {
                title: 'Update when it changes',
                detail: 'Projects show when the library has a newer version',
              },
            ]}
            action={
              <Button variant="primary" icon="plus" onClick={() => v.openAdd('pick')}>
                Add from a project
              </Button>
            }
          />
        </main>
      ) : (
        <main className="flex min-h-0 flex-1">
          <LibraryFilters items={v.items} filters={filters} onChange={setFilters} />
          <section
            aria-label="Models"
            className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
          >
            <LibraryStats stats={v.stats} />
            <div className="flex items-center gap-3">
              <h2 className="text-panel-title">{filtered ? 'Models found' : 'All models'}</h2>
              <span className="text-meta text-fg-mute">
                {filtered ? `${v.shown.length} of ${count(v.items.length)}` : count(v.items.length)}{' '}
                · {SORT_NOTE[sort]}
              </span>
              <span className="flex-1" />
              <SegmentedControl label="Sort" options={SORTS} value={sort} onChange={setSort} />
            </div>
            {v.error ? (
              <p role="alert" className="text-meta text-danger">
                {v.error}
              </p>
            ) : null}
            {v.shown.length === 0 && v.loaded ? (
              <p className="py-10 text-center text-fg-mute">
                No model matches. Clear the search or pick All models.
              </p>
            ) : layout === 'grid' ? (
              <ul className="grid list-none grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
                {cards}
              </ul>
            ) : (
              <ul className="flex list-none flex-col gap-1">{cards}</ul>
            )}
          </section>
          {item ? <LibraryDetail key={item.entry.id} item={item} /> : null}
        </main>
      )}
    </>
  );
}
