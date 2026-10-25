import { Icon, IconButton, StatusChip, Tooltip, cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { itemChip, itemMeta, itemTitle } from '@/state/inbox';
import { RowMenu } from './RowMenu';
import { RowThumb } from './RowThumb';

export interface InboxRowProps {
  item: InboxItem;
  selected: boolean;
  onSelect: () => void;
  /** Ticked for a batch redo. */
  checked?: boolean;
  /** Some model is ticked, so every select box shows. */
  selecting?: boolean;
  onCheck?: (on: boolean) => void;
  menuOpen?: boolean;
  onMenu?: () => void;
  onCloseMenu?: () => void;
}

/** One model in the inbox list: its picture, name, numbers and status, a select box and a menu. */
export default function InboxRow({
  item,
  selected,
  onSelect,
  checked = false,
  selecting = false,
  onCheck,
  menuOpen = false,
  onMenu,
  onCloseMenu,
}: InboxRowProps) {
  const chip = itemChip(item);
  const title = itemTitle(item);
  return (
    <div
      className={cn(
        'group/row relative rounded-card',
        selected ? 'bg-clay-soft shadow-[inset_0_0_0_1px_var(--clay-line)]' : 'hover:bg-hover',
        checked && !selected && 'shadow-[inset_0_0_0_1px_var(--clay-line)]'
      )}
    >
      <button
        type="button"
        aria-pressed={selected}
        onClick={onSelect}
        className="flex w-full cursor-pointer items-center gap-3 rounded-card p-2 pr-9 text-left"
      >
        <RowThumb item={item} />
        <span className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="truncate font-semibold">{title}</span>
          <span className="truncate font-mono text-[11.5px] leading-[14px] text-fg-mute">
            {itemMeta(item)}
          </span>
          <span>
            <StatusChip status={chip.status} label={chip.label} />
          </span>
        </span>
      </button>
      {onCheck ? (
        <Tooltip label="Select to redo together" side="right">
          <button
            type="button"
            role="checkbox"
            aria-checked={checked}
            aria-label={`Select ${title}`}
            onClick={() => onCheck(!checked)}
            className={cn(
              'absolute top-1 left-1 grid size-[20px] cursor-pointer place-items-center rounded-[6px] border-[1.5px] shadow-[0_1px_3px_rgba(0,0,0,0.35)] transition-opacity duration-120 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus',
              checked
                ? 'border-clay-text bg-clay text-clay-ink'
                : 'border-line-strong bg-panel/90 text-transparent hover:border-fg-mute',
              checked || selecting ? 'opacity-100' : 'opacity-0 group-hover/row:opacity-100'
            )}
          >
            <Icon name="check" size={12} />
          </button>
        </Tooltip>
      ) : null}
      {onMenu ? (
        <IconButton
          icon="dots-three"
          label={`More for ${title}`}
          size="sm"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onClick={onMenu}
          className={cn(
            'absolute top-1.5 right-1.5 focus-visible:opacity-100',
            menuOpen ? 'opacity-100' : 'opacity-0 group-hover/row:opacity-100'
          )}
        />
      ) : null}
      {menuOpen && onCloseMenu ? (
        <div className="absolute top-9 right-1 z-20">
          <RowMenu item={item} onClose={onCloseMenu} />
        </div>
      ) : null}
    </div>
  );
}
