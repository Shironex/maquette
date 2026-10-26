import { EmptyState, Icon, cn } from '@maquette/ui';
import type { DeviceId } from '@/state/page';
import { PageFrame } from '../PageFrame';
import { useCompareBoard } from './CompareBoard.hooks';
import type { CompareColumnView } from './CompareBoard.types';

interface CompareColumnProps {
  column: CompareColumnView;
  device: DeviceId;
}

function CompareColumn({ column, device }: CompareColumnProps) {
  const phone = device === 'phone';
  const rows = column.rows.map(r => (
    <div
      key={r.label}
      className="flex h-[31px] items-center justify-between gap-2 border-t border-line text-[12px]"
    >
      <span className="truncate text-fg-mute">{r.label}</span>
      <span className="flex items-baseline gap-1.5 font-mono text-[12px] whitespace-nowrap text-fg">
        {r.value}
        {r.change ? (
          <small
            className={cn(
              'text-[11px]',
              r.better === true ? 'text-sage' : r.better === false ? 'text-danger' : 'text-fg-mute'
            )}
          >
            {r.change}
          </small>
        ) : null}
      </span>
    </div>
  ));
  return (
    <section
      aria-label={column.title}
      className={cn('flex min-h-0 min-w-0 flex-1 flex-col gap-2', phone && 'max-w-[256px]')}
    >
      <header className="flex min-h-[38px] items-start justify-between gap-2">
        <div className="flex min-w-0 flex-col">
          <h2
            className={cn(
              'm-0 text-[13px] leading-[18px] font-semibold',
              column.next ? 'text-clay-text' : 'text-fg'
            )}
          >
            {column.title}
          </h2>
          <span className="truncate text-[12px] leading-4 text-fg-mute">{column.sub}</span>
        </div>
        {column.next ? (
          <span className="inline-flex h-[22px] flex-none items-center gap-1 text-[11.5px] font-medium text-fg">
            <Icon name="check" size={12} className="text-fg-dim" />
            Ships next
          </span>
        ) : null}
      </header>
      <PageFrame
        id={column.id}
        src={column.src}
        image={column.image}
        address={column.address}
        badge={column.badge}
        device={device}
        fit="width"
        compact
        label={`${column.title}, ${column.sub}`}
        className="flex-initial"
      >
        {column.empty ? (
          <p className="m-0 max-w-[220px] p-4 text-center text-meta text-fg-mute">{column.empty}</p>
        ) : null}
      </PageFrame>
      <div className="flex flex-none flex-col pt-1">{rows}</div>
    </section>
  );
}

/** The site now, this run and the page without WebGL, side by side at one device width. */
export default function CompareBoard() {
  const { columns, device } = useCompareBoard();
  const phone = device === 'phone';
  if (columns.length === 0) {
    return <EmptyState icon="columns" title="Nothing to compare yet" />;
  }
  const shown = columns.map(c => <CompareColumn key={c.id} column={c} device={device} />);
  return (
    <div className={cn('flex min-h-0 flex-1 gap-4', phone ? 'justify-center' : 'justify-stretch')}>
      {shown}
    </div>
  );
}
