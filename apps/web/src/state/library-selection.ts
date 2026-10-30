/**
 * What a Library selection has in common: the tags on every picked model
 * and the ones only some carry, for the Tag popover.
 */
import type { LibraryItem } from '@/ipc/client';

export interface SelectionTags {
  /** On every picked model, by name. */
  all: string[];
  /** On some of them, with how many, most first. */
  some: [string, number][];
}

export function selectionTags(items: readonly LibraryItem[]): SelectionTags {
  const counts = new Map<string, number>();
  for (const tag of items.flatMap(i => i.entry.tags)) counts.set(tag, (counts.get(tag) ?? 0) + 1);
  const all: string[] = [];
  const some: [string, number][] = [];
  for (const [tag, n] of counts) {
    if (n === items.length) all.push(tag);
    else some.push([tag, n]);
  }
  all.sort((a, b) => a.localeCompare(b, 'en'));
  some.sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], 'en'));
  return { all, some };
}

/** A typed tag as it is stored: trimmed and lowercase. */
export const cleanTag = (text: string) => text.trim().toLowerCase();

/**
 * The tags offered for what was typed: every library tag that contains it,
 * each with how many picked models carry it.
 */
export function tagMatches(
  typed: string,
  known: readonly string[],
  picked: readonly LibraryItem[]
): { tag: string; on: number }[] {
  const text = cleanTag(typed);
  if (!text) return [];
  return known
    .filter(t => t.includes(text))
    .slice(0, 6)
    .map(tag => ({ tag, on: picked.filter(i => i.entry.tags.includes(tag)).length }));
}
