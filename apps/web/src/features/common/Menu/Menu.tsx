import { cn } from '@maquette/ui';
import { useMenu } from './Menu.hooks';
import type { MenuProps } from './Menu.types';
import { MenuRow } from './MenuRow';
import { startsSection } from './menuKeys';
import { MENU_PANEL, MENU_SEPARATOR } from './menuStyles';

/**
 * A small popup list. Arrow keys, Home and End move, Escape and a click
 * outside close it, and so does a pick. Sections are parted by a line, and
 * an item with a submenu opens it beside itself.
 */
export default function Menu({ label, items, onClose, header, className }: MenuProps) {
  const { ref, onKeyDown } = useMenu(onClose);
  const rows = items.map((item, i) => (
    <div key={item.id} className="contents">
      {startsSection(items, i) ? <div role="separator" className={MENU_SEPARATOR} /> : null}
      <MenuRow item={item} onClose={onClose} />
    </div>
  ));
  return (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      onKeyDown={onKeyDown}
      className={cn('min-w-[180px]', MENU_PANEL, className)}
    >
      {header}
      {rows}
    </div>
  );
}
