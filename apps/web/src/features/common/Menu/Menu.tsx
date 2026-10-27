import { createPortal } from 'react-dom';
import { cn } from '@maquette/ui';
import { useMenu } from './Menu.hooks';
import type { MenuProps } from './Menu.types';
import { MenuRow } from './MenuRow';
import { startsSection } from './menuKeys';
import { MENU_PANEL, MENU_SEPARATOR } from './menuStyles';

/**
 * A small popup list, fixed in the window and kept inside it. Arrow keys, Home and End move, Escape and a click
 * outside close it, and so does a pick. Sections are parted by a line, and
 * an item with a submenu opens it beside itself.
 */
export default function Menu({ label, items, onClose, header, className }: MenuProps) {
  const { ref, onKeyDown, place } = useMenu(onClose);
  const rows = items.map((item, i) => (
    <div key={item.id} className="contents">
      {startsSection(items, i) ? <div role="separator" className={MENU_SEPARATOR} /> : null}
      <MenuRow item={item} onClose={onClose} />
    </div>
  ));
  const panel = (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      onKeyDown={onKeyDown}
      style={
        place
          ? { left: place.left, top: place.top, width: place.width, maxHeight: place.maxHeight }
          : undefined
      }
      className={cn(
        'min-w-[180px]',
        MENU_PANEL,
        place ? 'fixed z-50 overflow-y-auto overscroll-contain' : cn('invisible', className)
      )}
    >
      {header}
      {rows}
    </div>
  );
  // Until it is measured it sits where its caller put it, unseen; then it
  // moves to the body, where no scrolling or clipping parent can cut it.
  return place ? createPortal(panel, document.body) : panel;
}
