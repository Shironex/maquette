import { describe, expect, it } from 'vitest';
import type { LibraryView } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { isEmptyQuery, matchesQuery, parseQuery } from './library-query';

async function scale(): Promise<LibraryView> {
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    library: 'scale',
  });
  return (await handle('library_get')) as LibraryView;
}

const find = (view: LibraryView, text: string) =>
  view.items.filter(i => matchesQuery(i, parseQuery(text))).map(i => i.entry.id);

describe('the library search', () => {
  it('reads words, tag:, in:, by: and triangle bounds', () => {
    expect(parseQuery('stone TAG:light in:frostveil by:meshy <5k tris>1.5k')).toEqual({
      words: ['stone'],
      tags: ['light'],
      projects: ['frostveil'],
      sources: ['meshy'],
      over: 1_500,
      under: 5_000,
    });
    expect(isEmptyQuery(parseQuery('   '))).toBe(true);
  });

  it('finds by name, tag, project, source and triangles together', async () => {
    const view = await scale();
    expect(find(view, 'street lamp')).toEqual(['street-lamp', 'street-lamp-2']);
    expect(find(view, 'tag:light lamp')).toEqual(['street-lamp', 'street-lamp-2']);
    expect(find(view, 'in:cathedral')).toEqual(['crystal-lantern']);
    expect(find(view, 'by:built veil')).toEqual(['veil-platform']);
    const light = find(view, '<2k');
    expect(light.length).toBeGreaterThan(0);
    for (const id of light) {
      const tris = view.items.find(i => i.entry.id === id)!.entry.versions.at(-1)!.stats!.triangles;
      expect(tris).toBeLessThan(2_000);
    }
  });
});
