import { cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { creditLine, itemLine, usedLine } from '@/state/library';
import { LibraryPicture } from '../LibraryPicture';

export interface LibraryCardProps {
  item: LibraryItem;
  selected: boolean;
  onSelect: () => void;
}

/** One library model in the grid: picture, name, version and source, tags, uses and credits. */
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
    <button
      type="button"
      aria-pressed={selected}
      aria-label={item.entry.name}
      onClick={onSelect}
      className={cn(
        'flex w-full cursor-pointer flex-col overflow-hidden rounded-card border border-line bg-raised text-left transition-colors duration-120 hover:border-line-strong',
        selected && 'border-clay-line ring-2 ring-clay/25'
      )}
    >
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
    </button>
  );
}
