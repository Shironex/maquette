import { StatusChip, cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { itemChip, itemMeta, itemTitle } from '@/state/inbox';
import { RowThumb } from './RowThumb';

export interface InboxRowProps {
  item: InboxItem;
  selected: boolean;
  onSelect: () => void;
}

/** One model in the inbox list: its picture, name, numbers and status. */
export default function InboxRow({ item, selected, onSelect }: InboxRowProps) {
  const chip = itemChip(item);
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'flex w-full cursor-pointer items-center gap-3 rounded-card p-2 text-left',
        selected ? 'bg-clay-soft shadow-[inset_0_0_0_1px_var(--clay-line)]' : 'hover:bg-hover'
      )}
    >
      <RowThumb item={item} />
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-semibold">{itemTitle(item)}</span>
        <span className="truncate font-mono text-[11.5px] leading-[14px] text-fg-mute">
          {itemMeta(item)}
        </span>
        <span>
          <StatusChip status={chip.status} label={chip.label} />
        </span>
      </span>
    </button>
  );
}
