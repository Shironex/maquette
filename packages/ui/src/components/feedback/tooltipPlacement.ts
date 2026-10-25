export type TooltipSide = 'top' | 'bottom' | 'left' | 'right';

interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

interface Size {
  width: number;
  height: number;
}

export interface TooltipPlacement {
  top: number;
  left: number;
  /** The side it ended up on, after a flip. */
  side: TooltipSide;
}

/** The gap between the trigger and the tooltip. */
export const TOOLTIP_GAP = 8;
/** How close the tooltip may come to a window edge. */
export const EDGE_MARGIN = 8;

const OPPOSITE: Record<TooltipSide, TooltipSide> = {
  top: 'bottom',
  bottom: 'top',
  left: 'right',
  right: 'left',
};

/** The room between the trigger and the window edge on one side. */
function room(trigger: Box, side: TooltipSide, viewport: Size): number {
  if (side === 'top') return trigger.top;
  if (side === 'bottom') return viewport.height - trigger.top - trigger.height;
  if (side === 'left') return trigger.left;
  return viewport.width - trigger.left - trigger.width;
}

function clamp(value: number, size: number, limit: number): number {
  const max = limit - size - EDGE_MARGIN;
  return Math.max(EDGE_MARGIN, Math.min(value, max));
}

/**
 * Where a tooltip of `tip` size goes beside `trigger`, in window pixels:
 * on the preferred side when it fits, else on the opposite side when that
 * has more room, then pushed along the edge so it never leaves the window.
 */
export function tooltipPlacement(
  trigger: Box,
  tip: Size,
  preferred: TooltipSide,
  viewport: Size
): TooltipPlacement {
  const vertical = preferred === 'top' || preferred === 'bottom';
  const need = (vertical ? tip.height : tip.width) + TOOLTIP_GAP + EDGE_MARGIN;
  const flipped = OPPOSITE[preferred];
  const side =
    room(trigger, preferred, viewport) >= need ||
    room(trigger, preferred, viewport) >= room(trigger, flipped, viewport)
      ? preferred
      : flipped;

  if (side === 'top' || side === 'bottom') {
    const top =
      side === 'top'
        ? trigger.top - TOOLTIP_GAP - tip.height
        : trigger.top + trigger.height + TOOLTIP_GAP;
    const left = trigger.left + trigger.width / 2 - tip.width / 2;
    return {
      side,
      top: clamp(top, tip.height, viewport.height),
      left: clamp(left, tip.width, viewport.width),
    };
  }
  const left =
    side === 'left'
      ? trigger.left - TOOLTIP_GAP - tip.width
      : trigger.left + trigger.width + TOOLTIP_GAP;
  const top = trigger.top + trigger.height / 2 - tip.height / 2;
  return {
    side,
    top: clamp(top, tip.height, viewport.height),
    left: clamp(left, tip.width, viewport.width),
  };
}
