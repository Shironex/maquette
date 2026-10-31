import { SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { RigRecord, RigRow } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
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
  const flagged = row.defects.length > 0;
  const pick = { front: row.front, back: row.back, xray: row.xray, both: row.front }[view];
  const pins = row.defects.map(n => (
    <Pin key={n} number={n} amber={record.defects[n - 1]?.kind === 'needs-yes'} />
  ));
  return (
    <figure className="m-0 min-w-0">
      <div className="relative">
        {view === 'both' ? (
          <div className="grid h-[132px] grid-cols-2 gap-px">
            <Tile url={fileUrl(row.front)} alt={`${row.label}, front`} flagged={flagged} />
            <Tile url={fileUrl(row.side)} alt={`${row.label}, side`} flagged={flagged} />
          </div>
        ) : (
          <div className="h-[132px]">
            <Tile url={fileUrl(pick)} alt={`${row.label}, ${view}`} flagged={flagged} />
          </div>
        )}
        <span className="absolute top-1.5 left-1.5 flex gap-1">{pins}</span>
      </div>
      <figcaption className="mt-1 flex items-baseline justify-between gap-2 font-mono text-meta">
        <span className="text-fg-dim">{row.label}</span>
        <span className={cn('truncate', flagged ? 'text-clay-text' : 'text-fg-mute')}>
          {row.note ?? 'looks fine'}
        </span>
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
          <p className="m-0 mt-0.5 text-fg-mute">{rows.length} poses, rendered on a copy</p>
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
      <div className="grid flex-1 grid-cols-3 content-start gap-x-3 gap-y-3">{cells}</div>
      {record.stats.length > 0 ? <Stats stats={record.stats} /> : null}
    </section>
  );
}

/** The right column of the first board: what is wrong, by place. */
export function DefectsAside({ record }: { record: RigRecord }) {
  const fine = (record.before?.rows ?? []).filter(r => r.defects.length === 0).map(r => r.label);
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
        <span className="text-meta text-fg-mute">{record.defects.length} found</span>
      </div>
      {cards}
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
