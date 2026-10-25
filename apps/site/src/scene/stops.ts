/** The tour's stops in order: the hero frame, then Concept to Ship. Kept tiny for the page script. */
export const STOP_IDS = ['hero', 'concept', 'refs', 'model', 'blender', 'web', 'ship'] as const;
export type StopId = (typeof STOP_IDS)[number];
