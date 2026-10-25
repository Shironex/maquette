import { Icon, Tooltip, cn } from '@maquette/ui';
import { useMenu } from './Menu.hooks';
import type { MenuProps } from './Menu.types';

/** A small popup list. Closes on Escape, on a click outside and after a pick. */
export default function Menu({ label, items, onClose, header, className }: MenuProps) {
  const ref = useMenu(onClose);
  const rows = items.map(item => {
    const explained = Boolean(item.disabled && item.hint);
    return (
      <Tooltip key={item.id} label={item.hint} side="right" disabled={!item.hint}>
        <button
          type="button"
          role="menuitem"
          disabled={item.disabled && !explained}
          aria-disabled={explained || undefined}
          onClick={() => {
            if (explained) return;
            item.onSelect();
            if (!item.keepOpen) onClose();
          }}
          className="flex w-full cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-left text-[13px] hover:bg-hover focus-visible:bg-hover focus-visible:outline-none active:bg-line disabled:cursor-default disabled:text-fg-mute disabled:hover:bg-transparent aria-disabled:cursor-default aria-disabled:text-fg-mute aria-disabled:hover:bg-transparent"
        >
          {item.icon ? <Icon name={item.icon} size={14} className="text-fg-dim" /> : null}
          {item.label}
        </button>
      </Tooltip>
    );
  });
  return (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      className={cn(
        'min-w-[180px] rounded-card border border-line-strong bg-panel p-1 shadow-window',
        className
      )}
    >
      {header}
      {rows}
    </div>
  );
}
