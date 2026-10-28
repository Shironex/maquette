import { cn, type WindowItemProps } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { creditLine, itemLine, numbersLine, usedLine } from '@/state/library';
import { LibraryPicture } from '../LibraryPicture';

export interface LibraryRowProps {
  item: LibraryItem;
  selected: boolean;
  onSelect: () => void;
  /** Its measuring props, in a windowed list. */
  position?: WindowItemProps;
}

/** One library model in the list: the card's lines in one row, with its numbers. */
export default function LibraryRow({ item, selected, onSelect, position }: LibraryRowProps) {
  const credits = creditLine(item);
  return (
    <li {...position}>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={item.entry.name}
        onClick={onSelect}
        className={cn(
          'flex w-full cursor-pointer items-center gap-3 rounded-card p-2 text-left hover:bg-hover',
          selected && 'bg-clay-soft shadow-[inset_0_0_0_1px_var(--clay-line)] hover:bg-clay-soft'
        )}
      >
        <LibraryPicture item={item} className="size-12 flex-none rounded-control" />
        <span className="flex min-w-0 flex-[2] flex-col gap-0.5">
          <span className="truncate font-semibold">{item.entry.name}</span>
          <span className="truncate font-mono text-[11.5px] text-fg-mute">{itemLine(item)}</span>
        </span>
        <span className="min-w-0 flex-[2] truncate text-meta text-fg-mute">
          {item.entry.tags.join(', ')}
        </span>
        <span className="min-w-0 flex-[2] truncate font-mono text-[11.5px] text-fg-mute">
          {numbersLine(item) ?? ''}
        </span>
        <span className="w-24 flex-none text-meta text-fg-dim">{usedLine(item)}</span>
        <span
          className={cn(
            'w-36 flex-none text-right text-meta',
            credits.saved ? 'text-sage' : 'text-fg-mute'
          )}
        >
          {credits.text}
        </span>
      </button>
    </li>
  );
}
