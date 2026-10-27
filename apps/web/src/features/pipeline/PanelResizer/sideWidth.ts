/** The width of the runner's side panel (the compression presets), in CSS pixels. */
export const SIDE_WIDTH = { min: 380, initial: 560, max: 900, step: 24 } as const;
/** What the log beside it keeps at least. */
export const LOG_MIN = 360;

/** `width` kept inside the panel's limits and the room the row has (`room`, when known). */
export function clampWidth(width: number, room?: number): number {
  const max = room ? Math.min(SIDE_WIDTH.max, room - LOG_MIN) : SIDE_WIDTH.max;
  return Math.round(Math.max(SIDE_WIDTH.min, Math.min(Math.max(SIDE_WIDTH.min, max), width)));
}

/** The width a key asks for, or null for a key that does not move it. */
export function keyWidth(key: string, width: number): number | null {
  // The panel sits on the right, so left makes it wider.
  if (key === 'ArrowLeft') return width + SIDE_WIDTH.step;
  if (key === 'ArrowRight') return width - SIDE_WIDTH.step;
  if (key === 'Home') return SIDE_WIDTH.max;
  if (key === 'End') return SIDE_WIDTH.min;
  return null;
}
