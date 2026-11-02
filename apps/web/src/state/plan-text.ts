/**
 * The one set of characters a canvas trims from the ends of its name,
 * titles, addresses and labels, the same set the core trims
 * (`crates/maquette-model/src/plans/text.rs`). JavaScript's `trim` and
 * Rust's differ (U+FEFF and U+0085), and a canvas trimmed differently on
 * each side would read as changed and save again on every open.
 */

/** White space as Rust's `trim` and JavaScript's `trim` know it, together. */
export const TRIMMED: readonly string[] = [
  '\u0009',
  '\u000A',
  '\u000B',
  '\u000C',
  '\u000D',
  ' ',
  '\u0085',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  ' ',
  '　',
  '﻿',
];

const SET = new Set(TRIMMED);

/** `text` without {@link TRIMMED} characters at either end. */
export function trimText(text: string): string {
  let start = 0;
  let end = text.length;
  while (start < end && SET.has(text.charAt(start))) start += 1;
  while (end > start && SET.has(text.charAt(end - 1))) end -= 1;
  return text.slice(start, end);
}
