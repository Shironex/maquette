import { Fragment, type MouseEvent } from 'react';
import { Icon, cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { creditLine, itemParts, usedLine } from '@/state/library';
import { CARD, CARD_BUTTON } from '@/features/common/cardButton';
import { LibraryPicture } from '../LibraryPicture';
import { SelectBox } from '../SelectBox';
import type { ModelFlag } from '../LibraryView/LibraryView.types';

export interface LibraryCardProps {
  item: LibraryItem;
  selected: boolean;
  /** A click on the card; it carries Shift, Cmd and Ctrl for the selection. */
  onSelect: (e?: MouseEvent) => void;
  /** Opens the model in the 3D quick view, from a click on the picture. */
  onInspect?: () => void;
  /** In the bulk selection, and how its select box is clicked. */
  checked?: boolean;
  selecting?: boolean;
  onCheck?: (on: boolean, range: boolean) => void;
  /** A look-alike or same name pair it is in. */
  flag?: ModelFlag | null;
}

const FLAG_LABEL: Record<ModelFlag, string> = {
  'same-look': 'look-alike',
  'same-name': 'same name',
};

/**
 * One library model in the grid: picture, name, version, source and
 * triangles, tags, uses and credits. The button lies over the card and is
 * named after the model alone, so its name holds all the text it covers
 * (WCAG 2.5.3). The picture lies over that button and opens the 3D quick
 * view; the select box sits on the picture's corner.
 */
export default function LibraryCard(props: LibraryCardProps) {
  const { item, selected, onSelect, onInspect, checked = false, selecting = false } = props;
  const credits = creditLine(item);
  // Each part keeps together, so a long line ("Meshy, rigged") wraps between parts instead of being cut.
  const line = itemParts(item).map((part, n) => (
    <Fragment key={n}>
      {n ? ' · ' : null}
      <span className="whitespace-nowrap">{part}</span>
    </Fragment>
  ));
  const tags = item.entry.tags.map(tag => (
    <span
      key={tag}
      className="rounded-pill border border-line px-1.5 font-mono text-[11px] leading-[18px] text-fg-mute"
    >
      {tag}
    </span>
  ));
  return (
    <div
      className={cn(
        CARD,
        'group/model flex flex-col overflow-hidden',
        (selected || checked) && 'border-clay-line',
        checked ? 'ring-2 ring-clay/35' : selected && 'ring-2 ring-clay/25'
      )}
    >
      <button
        type="button"
        aria-pressed={selected}
        aria-label={item.entry.name}
        onClick={e => onSelect(e)}
        className={CARD_BUTTON}
      />
      <div className="relative">
        <LibraryPicture
          item={item}
          onInspect={
            onInspect &&
            (() => {
              onSelect();
              onInspect();
            })
          }
          className={cn('aspect-[4/3] w-full border-b border-line', onInspect && 'z-20')}
        />
        {props.onCheck ? (
          <SelectBox
            name={item.entry.name}
            checked={checked}
            selecting={selecting}
            onCheck={props.onCheck}
            className="absolute top-2 left-2 z-30"
          />
        ) : null}
        {props.flag ? (
          <span className="pointer-events-none absolute top-2 right-2 z-30 flex items-center gap-1 rounded-pill bg-panel/90 px-2 text-[11px] leading-5 text-amber shadow-[0_1px_3px_rgba(0,0,0,0.35)]">
            <Icon name={props.flag === 'same-look' ? 'copy' : 'note-pencil'} size={11} />
            {FLAG_LABEL[props.flag]}
          </span>
        ) : null}
      </div>
      <span className="flex flex-col gap-1.5 p-3">
        <span className="truncate font-semibold">{item.entry.name}</span>
        <span className="line-clamp-2 font-mono text-[11.5px] leading-[14px] text-fg-mute">
          {line}
        </span>
        {tags.length ? (
          <span className="flex flex-wrap gap-1">{tags}</span>
        ) : (
          <span className="text-[11px] leading-[18px] text-amber">untagged</span>
        )}
        <span className="flex items-center justify-between gap-2 text-meta">
          <span className="text-fg-dim">{usedLine(item)}</span>
          <span className={credits.saved ? 'text-sage' : 'text-fg-mute'}>{credits.text}</span>
        </span>
      </span>
    </div>
  );
}
