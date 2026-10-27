import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { menuItems, moveFocus } from '../menuKeys';

/** How wide a submenu is, to tell whether it fits to the right. */
export const SUBMENU_WIDTH = 168;

/**
 * An item's submenu: opened by hover, a click or ArrowRight, closed by
 * leaving it, ArrowLeft or Escape. Opened from the keyboard it focuses its
 * first item, and closing it from the keyboard hands focus back. It opens
 * to the left when the window has no room on the right.
 */
export function useSubmenu(enabled: boolean) {
  const [open, setOpen] = useState<'pointer' | 'keys' | null>(null);
  const [flip, setFlip] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open === 'keys' && menu.current) menuItems(menu.current)[0]?.focus();
  }, [open]);

  const show = (how: 'pointer' | 'keys') => {
    if (!enabled) return;
    const right = button.current?.getBoundingClientRect().right ?? 0;
    setFlip(right + SUBMENU_WIDTH > window.innerWidth);
    setOpen(how);
  };
  const hide = () => setOpen(null);

  return {
    open: open !== null,
    flip,
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
