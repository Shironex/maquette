import { useEffect, useRef, type ReactNode } from 'react';
import { Icon, cn, type IconName } from '@maquette/ui';

export interface MenuItem {
  id: string;
  label: string;
  icon?: IconName;
  disabled?: boolean;
  onSelect: () => void;
}

/** A small popup list. Closes on Escape, on a click outside and after a pick. */
export function Menu({
  label,
  items,
  onClose,
  header,
  className,
}: {
  label: string;
  items: readonly MenuItem[];
  onClose: () => void;
  header?: ReactNode;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>('button:not([disabled])')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const onDown = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onDown);
    };
  }, [onClose]);
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
      {items.map(item => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          onClick={() => {
            item.onSelect();
            onClose();
          }}
          className="flex w-full cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-left text-[13px] hover:bg-hover focus-visible:bg-hover focus-visible:outline-none disabled:cursor-default disabled:text-fg-mute disabled:hover:bg-transparent"
        >
          {item.icon ? <Icon name={item.icon} size={14} className="text-fg-dim" /> : null}
          {item.label}
        </button>
      ))}
    </div>
  );
}
