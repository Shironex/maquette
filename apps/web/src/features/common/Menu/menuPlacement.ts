/** The space kept between a menu and the window's edge. */
export const EDGE = 8;

/** The gap kept between a menu and what it opens from, when it has none of its own. */
const MIN_GAP = 4;

export interface MenuPlace {
  left: number;
  top: number;
  width: number;
  /** The most the menu may be tall; it scrolls inside when it is taller. */
  maxHeight: number;
}

interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/**
 * Where a menu stands in the window, fixed. `natural` is the box it would
 * have where its caller put it, `anchor` the box it opens from (the
 * trigger's wrapper). It is kept EDGE inside the window, shifted sideways
 * when it would cross an edge, and flipped above the anchor when there is
 * more room there than below. A menu taller than the room is capped and
 * scrolls inside.
 */
export function placeMenu(
  natural: Box,
  anchor: Box,
  view: { width: number; height: number }
): MenuPlace {
  const width = Math.min(natural.right - natural.left, view.width - 2 * EDGE);
  const height = natural.bottom - natural.top;
  const left = Math.max(EDGE, Math.min(natural.left, view.width - width - EDGE));
  const below = view.height - EDGE - natural.top;
  const gap = Math.max(MIN_GAP, natural.top - anchor.bottom);
  const above = anchor.top - gap - EDGE;
  if (height <= below || below >= above) {
    return { left, top: Math.max(EDGE, natural.top), width, maxHeight: Math.max(0, below) };
  }
  const shown = Math.min(height, above);
  return { left, top: anchor.top - gap - shown, width, maxHeight: shown };
}
