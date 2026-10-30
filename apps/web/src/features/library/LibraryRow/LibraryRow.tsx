import type { MouseEvent } from 'react';
import { cn, type WindowItemProps } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { creditLine, itemLine, numbersLine, usedLine } from '@/state/library';
import { LibraryPicture } from '../LibraryPicture';
import { SelectBox } from '../SelectBox';

export interface LibraryRowProps {
  item: LibraryItem;
  selected: boolean;
  /** A click on the row; it carries Shift, Cmd and Ctrl for the selection. */
  onSelect: (e: MouseEvent) => void;
  /** Its measuring props, in a windowed list. */
  position?: WindowItemProps;
  /** In the bulk selection, and how its select box is clicked. */
  checked?: boolean;
  selecting?: boolean;
  onCheck?: (on: boolean, range: boolean) => void;
}

/** One library model in the list: the card's lines in one row, with its numbers. */
export default function LibraryRow(props: LibraryRowProps) {
  const { item, selected, onSelect, position, checked = false, selecting = false } = props;
  const credits = creditLine(item);
  return (
    <li {...position} className="group/model flex items-center gap-1">
      {props.onCheck ? (
        <SelectBox
          name={item.entry.name}
          checked={checked}
          selecting={selecting}
          onCheck={props.onCheck}
        />
      ) : null}
      <button
        type="button"
        aria-pressed={selected}
        aria-label={item.entry.name}
        onClick={onSelect}
        className={cn(
          '@container flex min-w-0 flex-1 cursor-pointer items-center gap-3 rounded-card p-2 text-left hover:bg-hover',
          (selected || checked) &&
            'bg-clay-soft shadow-[inset_0_0_0_1px_var(--clay-line)] hover:bg-clay-soft'
        )}
      >
        <LibraryPicture item={item} className="size-12 flex-none rounded-control" />
        <span className="flex min-w-0 flex-[2] flex-col gap-0.5">
          <span className="truncate font-semibold">{item.entry.name}</span>
          <span className="truncate font-mono text-[11.5px] text-fg-mute">{itemLine(item)}</span>
        </span>
        <span className="hidden min-w-0 flex-[2] truncate text-meta text-fg-mute @3xl:block">
          {item.entry.tags.join(', ')}
        </span>
        <span className="hidden min-w-0 flex-[2] truncate font-mono text-[11.5px] text-fg-mute @3xl:block">
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
