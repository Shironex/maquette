import { cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { creditLine, itemLine, usedLine } from '@/state/library';
import { CARD, CARD_BUTTON } from '@/features/common/cardButton';
import { LibraryPicture } from '../LibraryPicture';

export interface LibraryCardProps {
  item: LibraryItem;
  selected: boolean;
  onSelect: () => void;
}

/**
 * One library model in the grid: picture, name, version and source, tags,
 * uses and credits. The button lies over the card and is named after the
 * model alone, so its name holds all the text it covers (WCAG 2.5.3).
 */
export default function LibraryCard({ item, selected, onSelect }: LibraryCardProps) {
  const credits = creditLine(item);
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
        'flex flex-col overflow-hidden',
        selected && 'border-clay-line ring-2 ring-clay/25'
      )}
    >
      <button
        type="button"
        aria-pressed={selected}
        aria-label={item.entry.name}
        onClick={onSelect}
        className={CARD_BUTTON}
      />
      <LibraryPicture item={item} className="aspect-[4/3] w-full border-b border-line" />
      <span className="flex flex-col gap-1.5 p-3">
        <span className="truncate font-semibold">{item.entry.name}</span>
        <span className="font-mono text-[11.5px] leading-[14px] text-fg-mute">
          {itemLine(item)}
        </span>
        {tags.length ? <span className="flex flex-wrap gap-1">{tags}</span> : null}
        <span className="flex items-center justify-between gap-2 text-meta">
          <span className="text-fg-dim">{usedLine(item)}</span>
          <span className={credits.saved ? 'text-sage' : 'text-fg-mute'}>{credits.text}</span>
        </span>
      </span>
    </div>
  );
}
