import { describe, expect, it } from 'vitest';
import type { LibraryItem } from '@/ipc/client';
import { cleanTag, selectionTags, tagMatches } from './library-selection';

const item = (id: string, tags: string[]) => ({ entry: { id, tags } }) as unknown as LibraryItem;

describe("a selection's tags", () => {
  const picked = [
    item('a', ['furniture', 'kitchen']),
    item('b', ['furniture']),
    item('c', ['furniture', 'prop']),
  ];

  it('parts the tags on every model from the ones on some', () => {
    expect(selectionTags(picked)).toEqual({
      all: ['furniture'],
      some: [
        ['kitchen', 1],
        ['prop', 1],
      ],
    });
  });

  it('offers the known tags that hold what was typed, with how many carry each', () => {
    expect(tagMatches(' KIT', ['kitchen', 'kit', 'light'], picked)).toEqual([
      { tag: 'kitchen', on: 1 },
      { tag: 'kit', on: 0 },
    ]);
    expect(tagMatches('', ['kitchen'], picked)).toEqual([]);
    expect(cleanTag('  Light ')).toBe('light');
  });
});
