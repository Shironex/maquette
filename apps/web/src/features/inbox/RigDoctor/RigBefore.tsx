import { EmptyState, SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { RigRecord, RigRow } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { cellAspect, isRunning, rowFlagged, rowNumbers } from '@/state/rig';
import { Enlarges } from './RigEnlarge';
import { useChoice } from './RigDoctor.hooks';
import { DefectCard, Pin, RunProgress, Stats, Tile } from './parts';

type View = 'both' | 'front' | 'back' | 'xray';

const VIEWS: readonly SegmentedOption<View>[] = [
  { value: 'both', label: 'Front and side' },
  { value: 'front', label: 'Front' },
  { value: 'back', label: 'Back' },
  { value: 'xray', label: 'X-ray bones' },
];

function Cell({ row, view, record }: { row: RigRow; view: View; record: RigRecord }) {
  const flagged = rowFlagged(row);
  const pick = { front: row.front, back: row.back, xray: row.xray, both: row.front }[view];
  const pins = row.defects.map(n => (
    <Pin key={n} number={n} amber={record.defects[n - 1]?.kind === 'needs-yes'} />
  ));
  const pictures =
    view === 'both'
      ? [
          { url: fileUrl(row.front), caption: 'Front' },
          { url: fileUrl(row.side), caption: 'Side' },
        ]
      : [{ url: fileUrl(pick), caption: VIEWS.find(v => v.value === view)?.label ?? view }];
  const tiles = pictures.map(p => (
    <Tile
      key={p.caption}
      url={p.url}
      alt={`${row.label}, ${p.caption.toLowerCase()}`}
      flagged={flagged}
      aspect={cellAspect(record)}
    />
  ));
  const verdict = row.note ?? row.look ?? 'looks fine';
  const tone = flagged ? 'text-clay-text' : row.look ? 'text-amber' : 'text-fg-mute';
  return (
    <figure className="m-0 min-w-0">
      <Enlarges
        title={row.label}
        detail={`${verdict}. ${rowNumbers(row)}`}
        pages={[{ label: view, pictures }]}
      >
        <div className="relative">
          <div className={cn('grid gap-px', view === 'both' && 'grid-cols-2')}>{tiles}</div>
          <span className="absolute top-1.5 left-1.5 flex gap-1">{pins}</span>
        </div>
      </Enlarges>
      <figcaption className="mt-1 font-mono text-meta">
        <span className="flex items-baseline justify-between gap-2">
          <span className="text-fg-dim">{row.label}</span>
          <span className={cn('text-right', tone)}>{verdict}</span>
        </span>
        <span className="block text-fg-mute">{rowNumbers(row)}</span>
      </figcaption>
    </figure>
  );
}

/** The centre of the first board: the before sheet, cell by cell. */
export function BeforeCentre({ record }: { record: RigRecord }) {
  const [view, setView] = useChoice<View>('both');
  const rows = record.before?.rows ?? [];
  const cells = rows.map(row => <Cell key={row.id} row={row} view={view} record={record} />);
  return (
    <section
      aria-label="Before sheet"
      className="flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto px-5 pt-4 pb-4"
    >
      <div className="flex flex-none items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="m-0 text-panel-title">
            Before sheet{' '}
            <span className="ml-1 rounded-[5px] bg-hover px-1.5 py-0.5 font-mono text-meta">
              v{record.source.version ?? 1}
            </span>
          </h2>
          <p className="m-0 mt-0.5 text-fg-mute">
            {rows.length > 0
              ? `${rows.length} poses, rendered on a copy`
              : isRunning(record)
                ? 'Rendering on a copy'
                : 'Not tested yet'}
          </p>
        </div>
        <SegmentedControl
          label="Sheet view"
          tone="neutral"
          options={VIEWS}
          value={view}
          onChange={setView}
        />
      </div>
      <RunProgress record={record} />
      {rows.length === 0 && !isRunning(record) ? (
        <div className="grid flex-1 place-items-center">
          <EmptyState
            icon="person-simple-walk"
            title="Not tested yet"
            body="The pose test renders each pose front and side on a copy and finds what tears. It runs when you start it."
          />
        </div>
      ) : null}
      <div
        className={cn(
          'grid flex-1 content-start gap-x-3 gap-y-3',
          view === 'both'
            ? 'grid-cols-[repeat(auto-fill,minmax(280px,1fr))]'
            : 'grid-cols-[repeat(auto-fill,minmax(190px,1fr))]'
        )}
      >
        {cells}
      </div>
      {record.stats.length > 0 ? <Stats stats={record.stats} /> : null}
    </section>
  );
}

/** The right column of the first board: what is wrong, by place. */
export function DefectsAside({ record }: { record: RigRecord }) {
  const rows = record.before?.rows ?? [];
  const tested = rows.length > 0;
  const fine = rows.filter(r => !rowFlagged(r) && !r.look).map(r => r.label);
  const looks = rows.filter(r => !rowFlagged(r) && r.look);
  const lookItems = looks.map(r => (
    <li key={r.id}>
      {r.label}: {r.look}
    </li>
  ));
  const cards = record.defects.map(d => (
    <DefectCard key={d.number} defect={d} on={d.number === 1} />
  ));
  return (
    <aside
      aria-label="Defects by place"
      className="flex w-[350px] flex-none flex-col gap-2.5 overflow-y-auto border-l border-line p-4 [&>*]:flex-none"
    >
      <div className="flex items-baseline justify-between">
        <h2 className="m-0 text-panel-title">Defects by place</h2>
        <span className="text-meta text-fg-mute">
          {tested ? `${record.defects.length} found` : isRunning(record) ? 'Testing' : 'Not tested'}
        </span>
      </div>
      {cards}
      {tested ? null : (
        <p className="m-0 text-meta text-fg-mute">
          {isRunning(record)
            ? 'What the pose test finds lands here as each pose finishes.'
            : 'What the pose test finds lands here, by place. Nothing has been checked yet.'}
        </p>
      )}
      {looks.length > 0 ? (
        <article className="rounded-[10px] border border-amber/40 p-3">
          <h4 className="m-0 font-medium text-amber">Look here</h4>
          <p className="m-0 mt-1 text-meta text-fg-mute">
            Real but small: too little of the surface to call a tear. Play these clips in Motion and
            decide.
          </p>
          <ul className="m-0 mt-1 list-none p-0 text-meta text-fg-dim">{lookItems}</ul>
        </article>
      ) : null}
      {fine.length > 0 ? (
        <article className="rounded-[10px] border border-line p-3">
          <h4 className="m-0 font-medium text-fg-dim">Looks fine</h4>
          <p className="m-0 mt-1 text-meta text-fg-mute">
            {fine.join(', ')} carry the body without tearing.
          </p>
        </article>
      ) : null}
    </aside>
  );
}
