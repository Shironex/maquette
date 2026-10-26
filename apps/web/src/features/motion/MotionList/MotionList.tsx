import { Button, Icon, SegmentedControl, cn } from '@maquette/ui';
import type { MotionFilter } from '@/state/motion';
import { useMotionList, type ListRow } from './MotionList.hooks';

const FILTERS: readonly { value: MotionFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'characters', label: 'Characters' },
  { value: 'props', label: 'Props' },
];

function Row({ row, current, onSelect }: { row: ListRow; current: boolean; onSelect: () => void }) {
  const { model } = row;
  return (
    <button
      type="button"
      aria-current={current ? 'true' : undefined}
      onClick={onSelect}
      className={cn(
        'flex w-full cursor-pointer items-center gap-2.5 rounded-card border border-transparent p-2 text-left hover:bg-hover',
        current && 'border-clay-line bg-clay-soft hover:bg-clay-soft'
      )}
    >
      <span className="grid size-11 flex-none place-items-center overflow-hidden rounded-[8px] bg-studio">
        {row.thumb ? (
          <img src={row.thumb} alt="" className="size-full object-cover" />
        ) : (
          <Icon
            name={model.rig ? 'person-simple-walk' : 'cube'}
            size={18}
            className="text-fg-mute"
          />
        )}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-[13px] leading-[17px] font-semibold">{model.name}</span>
        <span className="truncate font-mono text-[11.5px] leading-4 text-fg-mute">{row.line}</span>
      </span>
      {row.over ? (
        <span className="flex flex-none items-center gap-1 rounded-pill bg-danger-soft px-2 text-[11.5px] leading-5 text-danger">
          <Icon name="x-circle" size={12} />
          Over
        </span>
      ) : (
        <span className="flex-none font-mono text-[11.5px] leading-4 font-medium text-fg-dim">
          {row.size}
        </span>
      )}
    </button>
  );
}

function StillRow({
  row,
  current,
  onSelect,
}: {
  row: ListRow;
  current: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-current={current ? 'true' : undefined}
      onClick={onSelect}
      className={cn(
        'flex w-full cursor-pointer items-center gap-2.5 rounded-card border border-transparent p-2 text-left hover:bg-hover',
        current && 'border-clay-line bg-clay-soft hover:bg-clay-soft'
      )}
    >
      <span className="grid size-11 flex-none place-items-center rounded-[8px] bg-studio">
        <Icon name="cube" size={18} className="text-fg-mute" />
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-[13px] leading-[17px] font-medium text-fg-dim">
          {row.model.name}
        </span>
        <span className="truncate font-mono text-[11.5px] leading-4 text-fg-mute">{row.line}</span>
      </span>
    </button>
  );
}

/** The project's models: the ones that move first, then the ones that could. */
export default function MotionList() {
  const list = useMotionList();
  const moving = list.moving.map(row => (
    <Row
      key={row.model.asset}
      row={row}
      current={row.model.asset === list.asset}
      onSelect={() => list.select(row.model.asset)}
    />
  ));
  const still = list.still.map(row => (
    <StillRow
      key={row.model.asset}
      row={row}
      current={row.model.asset === list.asset}
      onSelect={() => list.select(row.model.asset)}
    />
  ));
  return (
    <section
      aria-label="Models that move"
      className="flex w-[272px] flex-none flex-col border-r border-line bg-panel"
    >
      <div className="flex flex-col gap-2.5 px-4 pt-[18px] pb-3">
        <div>
          <h2 className="text-panel-title">Motion</h2>
          <p className="text-meta text-fg-mute">
            {list.count} of {list.total} {list.total === 1 ? 'model moves' : 'models move'}
          </p>
        </div>
        {list.count > 0 ? (
          <SegmentedControl
            label="Show"
            options={FILTERS}
            value={list.filter}
            onChange={list.setFilter}
            block
          />
        ) : (
          <p className="pt-2 text-meta text-fg-dim">Animated characters and props show here.</p>
        )}
      </div>
      {list.count > 0 ? (
        <div className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-2 pb-2">
          {moving}
          {list.stillCount > 0 ? (
            <p className="px-2 pt-3.5 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
              No motion yet · {list.stillCount}
            </p>
          ) : null}
          {still}
          {list.hidden > 0 ? (
            <Button
              variant="ghost"
              size="sm"
              icon="caret-down"
              className="justify-start"
              onClick={list.expand}
            >
              Show {list.hidden} more
            </Button>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
