import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import { moveFocus } from './menuKeys';
import { placeMenu, type MenuPlace } from './menuPlacement';

/**
 * The menu's place in the window, and its keyboard. The menu first renders
 * where its caller puts it, unseen, to be measured; then it moves to a
 * fixed place inside the window (see `placeMenu`), where no scrolling or
 * clipping parent can cut it. It focuses the first enabled item, moves
 * between the items with the arrow keys, Home and End, and closes on
 * Escape, a click outside, or a scroll or resize of the page around it.
 */
export function useMenu(onClose: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<MenuPlace | null>(null);

  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const natural = menu.getBoundingClientRect();
    const anchor = menu.offsetParent?.getBoundingClientRect() ?? natural;
    setPlace(placeMenu(natural, anchor, { width: window.innerWidth, height: window.innerHeight }));
  }, []);

  const placed = place !== null;
  useEffect(() => {
    if (!placed) return;
    ref.current?.querySelector<HTMLButtonElement>('button:not([disabled])')?.focus();
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const onDown = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    // The menu is fixed, so a scrolled column would leave it behind.
    const onScroll = (e: Event) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onDown);
    window.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', onClose);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onDown);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onClose);
    };
  }, [onClose, placed]);

  const onKeyDown = (e: KeyboardEvent) => {
    if (ref.current && moveFocus(ref.current, e.key)) e.preventDefault();
  };
  return { ref, onKeyDown, place };
}
