/**
 * Part colours: one hue each, told apart by lightness too, readable on the
 * dark studio floor. Kept out of `parts.ts`, so the Parts tab's list shows
 * them without pulling three.js into the first bundle.
 */
export const PART_COLOURS: readonly string[] = [
  '#e0a458',
  '#6fa8dc',
  '#93c47d',
  '#c27ba0',
  '#f6d06f',
  '#76c5c0',
  '#d9776b',
  '#b4a7d6',
];

/** The colour of the `index`th part. */
export const partColour = (index: number): string =>
  PART_COLOURS[index % PART_COLOURS.length] ?? PART_COLOURS[0]!;
