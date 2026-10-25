import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type PointerEvent,
} from 'react';
import { tooltipPlacement, type TooltipPlacement, type TooltipSide } from './tooltipPlacement';

/** How long the pointer rests on a trigger before its tooltip shows. */
export const OPEN_DELAY = 500;
/** After a tooltip hides, a neighbour's shows at once for this long. */
export const SKIP_DELAY = 300;

/**
 * Every tooltip shares one delay: while one shows, or just after it hid,
 * moving to the next trigger shows that one at once instead of waiting
 * again, so a toolbar reads without flicker. Only one shows at a time.
 */
const group: { hideOpen: (() => void) | null; closedAt: number } = {
  hideOpen: null,
  closedAt: -Infinity,
};

/** Keyboard focus shows the tooltip; a click that focuses does not. */
function keyboardFocus(target: Element): boolean {
  try {
    return target.matches(':focus-visible');
  } catch {
    return true;
  }
}

interface TooltipOptions {
  side: TooltipSide;
  /** Forces it shown or hidden. */
  open?: boolean;
  disabled?: boolean;
  delay?: number;
  /** What it says, so a new label is measured again. */
  content: string;
}

/**
 * A tooltip's open state, its place in the window and the trigger's
 * handlers: it shows after a rest on hover (at once inside the delay
 * group) and on keyboard focus, and hides on leave, blur, a press,
 * Escape, scrolling what holds the trigger, and resizing.
 */
export function useTooltip({
  side,
  open: forced,
  disabled,
  delay = OPEN_DELAY,
  content,
}: TooltipOptions) {
  const id = useId();
  const anchor = useRef<HTMLSpanElement>(null);
  const bubble = useRef<HTMLSpanElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  // A press hides the tooltip until the pointer leaves.
  const pressed = useRef(false);
  const [open, setOpen] = useState(false);
  const [place, setPlace] = useState<TooltipPlacement | null>(null);
  const visible = !disabled && (forced ?? open);

  const close = useCallback(function close() {
    clearTimeout(timer.current);
    timer.current = undefined;
    setOpen(false);
    if (group.hideOpen !== close) return;
    group.hideOpen = null;
    group.closedAt = performance.now();
  }, []);

  const reveal = useCallback(() => {
    clearTimeout(timer.current);
    timer.current = undefined;
    if (group.hideOpen && group.hideOpen !== close) group.hideOpen();
    group.hideOpen = close;
    setOpen(true);
  }, [close]);

  const start = (now: boolean) => {
    if (disabled || pressed.current) return;
    const warm = group.hideOpen !== null || performance.now() - group.closedAt < SKIP_DELAY;
    if (now || warm) {
      reveal();
      return;
    }
    clearTimeout(timer.current);
    timer.current = setTimeout(reveal, delay);
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    // Only a scroll that moves the trigger; a log scrolling beside it does not.
    const onScroll = (e: Event) => {
      const trigger = anchor.current?.firstElementChild;
      if (trigger && e.target instanceof Node && e.target.contains(trigger)) close();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', close);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', close);
    };
  }, [open, close]);

  // Unmounting while shown lets the group go.
  useEffect(() => close, [close]);

  // Measured when it shows and when the label changes under it.
  useLayoutEffect(() => {
    const tip = bubble.current;
    // The anchor is display: contents, so the trigger is its first child.
    const trigger = anchor.current?.firstElementChild ?? anchor.current;
    if (!visible || !tip || !trigger) return;
    const next = tooltipPlacement(
      trigger.getBoundingClientRect(),
      tip.getBoundingClientRect(),
      side,
      {
        width: window.innerWidth,
        height: window.innerHeight,
      }
    );
    setPlace(now =>
      now && now.top === next.top && now.left === next.left && now.side === next.side ? now : next
    );
  }, [visible, side, content]);

  const handlers = {
    onPointerEnter: (e: PointerEvent) => {
      if (e.pointerType !== 'touch') start(false);
    },
    onPointerLeave: () => {
      pressed.current = false;
      close();
    },
    onPointerDown: () => {
      pressed.current = true;
      close();
    },
    onFocus: (e: FocusEvent) => {
      if (keyboardFocus(e.target)) start(true);
    },
    onBlur: close,
  };

  return { id, anchor, bubble, visible, place: visible ? place : null, handlers };
}
