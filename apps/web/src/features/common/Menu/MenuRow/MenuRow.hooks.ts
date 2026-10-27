import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { menuItems, moveFocus } from '../menuKeys';
import { EDGE } from '../menuPlacement';

/** How wide a submenu is, to tell whether it fits to the right. */
export const SUBMENU_WIDTH = 168;

/** About how tall one item is, to keep a long submenu inside the window. */
const ROW_HEIGHT = 32;

/**
 * Where the submenu's box stands in the window. The box starts at the item's
 * edge and pads out past the menu's border, so the pointer never leaves the
 * item on its way across.
 */
export interface SubmenuPlace {
  top: number;
  left: number;
  padLeft: number;
  padRight: number;
}

/** The gap between the menu and its submenu. */
const GAP = 2;

/**
 * Beside `menu` at the height of `item`: to the right, or to the left when
 * the window has no room there, and lifted to stay inside its bottom edge.
 * Fixed in the window, so a scrolling column around the menu cannot clip it.
 */
export function placeSubmenu(menu: DOMRect, item: DOMRect, rows: number): SubmenuPlace {
  const height = rows * ROW_HEIGHT + 10;
  const top = Math.max(EDGE, Math.min(item.top - 5, window.innerHeight - height - EDGE));
  if (menu.right + GAP + SUBMENU_WIDTH <= window.innerWidth) {
    return { top, left: item.right, padLeft: menu.right - item.right + GAP, padRight: 0 };
  }
  const padRight = item.left - menu.left + GAP;
  return { top, left: item.left - padRight - SUBMENU_WIDTH, padLeft: 0, padRight };
}

/**
 * An item's submenu: opened by hover, a click or ArrowRight, closed by
 * leaving it, ArrowLeft or Escape. Opened from the keyboard it focuses its
 * first item, and closing it from the keyboard hands focus back.
 */
export function useSubmenu(enabled: boolean, rows: number) {
  const [open, setOpen] = useState<'pointer' | 'keys' | null>(null);
  const [place, setPlace] = useState<SubmenuPlace>({ top: 0, left: 0, padLeft: 0, padRight: 0 });
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open === 'keys' && menu.current) menuItems(menu.current)[0]?.focus();
  }, [open]);

  const show = (how: 'pointer' | 'keys') => {
    const item = button.current;
    const menu = item?.closest('[role="menu"]');
    if (!enabled || !item || !menu) return;
    setPlace(placeSubmenu(menu.getBoundingClientRect(), item.getBoundingClientRect(), rows));
    setOpen(how);
  };
  const hide = () => setOpen(null);

  return {
    open: open !== null,
    place,
    button,
    menu,
    onPointerEnter: () => show('pointer'),
    onPointerLeave: hide,
    // A click from the keyboard (Enter or Space) has no pointer detail.
    onClick: (e: MouseEvent) => show(e.detail === 0 ? 'keys' : (open ?? 'pointer')),
    onButtonKey: (e: KeyboardEvent) => {
      if (e.key !== 'ArrowRight') return;
      e.preventDefault();
      show('keys');
    },
    onMenuKey: (e: KeyboardEvent) => {
      if (e.key === 'ArrowLeft' || e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        hide();
        button.current?.focus();
      } else if (menu.current && moveFocus(menu.current, e.key)) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
  };
}
