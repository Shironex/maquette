import { Button, EmptyState, SegmentedControl, TextField, TopBar } from '@maquette/ui';
import { StatTile } from '@/features/common/StatTile';
import { formatBytes } from '@/lib/format';
import { AnimationFilters } from '../AnimationFilters';
import { ClipCard } from '../ClipCard';
import { ClipDetail } from '../ClipDetail';
import { ClipRow } from '../ClipRow';
import { PressToggle } from '@/features/common/PressToggle';
import { useAnimationsView } from './AnimationsView.hooks';

const IMPORT_TIP = 'Add Mixamo FBX files or a pack zip; Blender reads them headless';

/**
 * The workspace's animation library: clips kept once outside any project,
 * skeleton only, that rigged models take by baking. Each preview is the
 * clip on its own skeleton, so no model is needed to look at one.
 */
export default function AnimationsView() {
  const v = useAnimationsView();
  const { filters, setFilters, layout, setLayout, item, select, stats } = v;
  const count = (n: number) => `${n} ${n === 1 ? 'clip' : 'clips'}`;
  const filtered = v.shown.length !== v.items.length;
  const cards = v.shown.map(i =>
    layout === 'grid' ? (
      <li key={i.clip.id}>
        <ClipCard
          item={i}
          selected={i.clip.id === item?.clip.id}
          onSelect={() => select(i.clip.id)}
        />
      </li>
    ) : (
      <ClipRow
        key={i.clip.id}
        item={i}
        selected={i.clip.id === item?.clip.id}
        onSelect={() => select(i.clip.id)}
      />
    )
  );
  const importButton = (primary: boolean) => (
    <Button
      variant={primary ? 'primary' : undefined}
      icon="download-simple"
      tooltip={IMPORT_TIP}
      onClick={v.openImport}
    >
      Import clips
    </Button>
  );

  return (
    <>
      <TopBar
        lead={
          <div className="flex items-baseline gap-2.5">
            <h1 className="text-panel-title">Animations</h1>
            <span className="text-meta text-fg-mute">Library · every project</span>
          </div>
        }
        right={
          <div className="flex items-center gap-2">
            <TextField
              type="search"
              icon="magnifying-glass"
              aria-label="Find a clip"
              placeholder="Find a clip, pack or tag"
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
              ]}
              value={layout}
              onChange={setLayout}
            />
            {importButton(false)}
          </div>
        }
      />
      {v.loaded && v.items.length === 0 ? (
        <main className="flex min-h-0 flex-1 items-center justify-center p-6">
          <EmptyState
            icon="person-simple"
            title="No clips yet"
            body="Clips live here once, skeleton only, for every project in this workspace. A rigged model takes one by baking it."
            steps={[
              {
                title: 'Import clips',
                detail: 'Mixamo FBX files or a whole pack zip',
              },
              {
                title: "Or save a model's clips",
                detail: 'Save clips to library in Motion, or from a Tripo job in the Inbox',
              },
            ]}
            action={importButton(true)}
          />
        </main>
      ) : (
        <main className="flex min-h-0 flex-1">
          <AnimationFilters items={v.items} filters={filters} onChange={setFilters} dir={v.dir} />
          <section
            aria-label="Clips"
            className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
          >
            <div className="flex gap-3">
              <StatTile
                label="Clips"
                value={String(stats.clips)}
                note={`${stats.packs} ${stats.packs === 1 ? 'pack' : 'packs'}, ${stats.singles} singles`}
              />
              <StatTile label="On disk" value={formatBytes(stats.disk)} note="skeleton only" />
            </div>
            <div className="flex items-center gap-3">
              <h2 className="text-panel-title">{filtered ? 'Clips found' : 'All clips'}</h2>
              <span className="text-meta text-fg-mute">
                {filtered ? `${v.shown.length} of ${count(v.items.length)}` : count(v.items.length)}{' '}
                · newest first
              </span>
              <span className="flex-1" />
              <PressToggle
                icon="path"
                label="Root motion"
                tooltip="Shows only clips that travel"
                pressed={filters.rootMotion}
                onChange={on => setFilters({ rootMotion: on })}
              />
              <PressToggle
                icon="repeat"
                label="Loops"
                tooltip="Shows only clips that loop"
                pressed={filters.loops}
                onChange={on => setFilters({ loops: on })}
              />
            </div>
            {v.error ? (
              <p role="alert" className="text-meta text-danger">
                {v.error}
              </p>
            ) : null}
            {v.shown.length === 0 && v.loaded ? (
              <p className="py-10 text-center text-fg-mute">
                No clip matches. Clear the search or pick All clips.
              </p>
            ) : layout === 'grid' ? (
              <ul className="grid list-none grid-cols-[repeat(auto-fill,minmax(164px,1fr))] gap-3">
                {cards}
              </ul>
            ) : (
              <ul className="flex list-none flex-col gap-1">{cards}</ul>
            )}
            {v.shown.length > 0 ? (
              <p className="text-meta text-fg-mute">
                Each preview is the clip on its own skeleton; no model is needed to look at a clip.
              </p>
            ) : null}
          </section>
          {item ? <ClipDetail key={item.clip.id} item={item} /> : null}
        </main>
      )}
    </>
  );
}
