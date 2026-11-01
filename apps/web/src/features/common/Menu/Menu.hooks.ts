import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
} from 'react';
import { clock } from '@/lib/clock';
import { moveFocus } from './menuKeys';
import { placeMenu, type MenuPlace } from './menuPlacement';

/**
 * How long after opening a click that counts as second of a double-click is
 * dropped. Long enough for a slow one; a single click is never held up.
 */
export const DOUBLE_CLICK_GUARD_MS = 600;

/**
 * The menu's place in the window, and its keyboard. The menu first renders
 * where its caller puts it, unseen, to be measured; then it moves to a
 * fixed place inside the window (see `placeMenu`), where no scrolling or
 * clipping parent can cut it. It focuses the first enabled item, moves
 * between the items with the arrow keys, Home and End, and closes on
 * Escape, a click outside, or a scroll or resize of the page around it.
 * Closed with focus inside it, it gives focus back to what opened it.
 * The second click of the double-click that opened it, landing on a row now
 * under the pointer, does nothing; a keyboard press has no click count.
 */
export function useMenu(onClose: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<MenuPlace | null>(null);
  const opener = useRef<HTMLElement | null>(null);
  const openedAt = useRef(0);

  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    openedAt.current = clock.nowMs();
    const active = document.activeElement;
    if (active instanceof HTMLElement && active !== document.body) opener.current = active;
    const natural = menu.getBoundingClientRect();
    const anchor = menu.offsetParent?.getBoundingClientRect() ?? natural;
    setPlace(placeMenu(natural, anchor, { width: window.innerWidth, height: window.innerHeight }));
  }, []);

  // Runs before a dialog the pick opened takes focus, so that dialog hands it back here too.
  useEffect(
    () => () => {
      const now = document.activeElement;
      if (now && now !== document.body && now.isConnected) return;
      if (opener.current?.isConnected) opener.current.focus();
    },
    []
  );

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
    window.addEventListener('resize', onClose);
    // A scroll that brought the trigger into view is still queued when the menu
    // opens, so the scroll listener waits two frames.
    let frame = requestAnimationFrame(() => {
      frame = requestAnimationFrame(() => window.addEventListener('scroll', onScroll, true));
    });
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onDown);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onClose);
    };
  }, [onClose, placed]);

  const onKeyDown = (e: KeyboardEvent) => {
    if (ref.current && moveFocus(ref.current, e.key)) e.preventDefault();
  };
  const onClickCapture = (e: MouseEvent) => {
    if (e.detail < 2 || clock.nowMs() - openedAt.current >= DOUBLE_CLICK_GUARD_MS) return;
    e.preventDefault();
    e.stopPropagation();
  };
  return { ref, onKeyDown, onClickCapture, place };
}
