import { describe, expect, it } from 'vitest';
import type { Asset, LibraryItem, LibraryView, ProjectManifest } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import {
  NO_FILTERS,
  NOT_USED,
  creditLine,
  filterItems,
  itemLine,
  libraryStats,
  pictureOf,
  projectCounts,
  sortItems,
  tagCounts,
  updateFor,
  useBlocked,
  useNote,
  usedLine,
} from './library';

async function demo(): Promise<LibraryView> {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, library: 'demo' });
  return (await handle('library_get')) as LibraryView;
}

const ids = (items: readonly LibraryItem[]) => items.map(i => i.entry.id);
const byId = (view: LibraryView, id: string) => view.items.find(i => i.entry.id === id)!;

describe('library filters', () => {
  it('searches names, tags and the projects that use a model', async () => {
    const { items } = await demo();
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'stone' }))).toEqual(['stone-lantern']);
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'rooftop' }))).toEqual([
      'vending-machine',
    ]);
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'street props' }))).toEqual([
      'vending-machine',
    ]);
  });

  it('filters by tag, source, format and the project that uses it', async () => {
    const { items } = await demo();
    expect(ids(filterItems(items, { ...NO_FILTERS, tag: 'garden' }))).toEqual([
      'stone-lantern',
      'sakura-tree',
    ]);
    expect(ids(filterItems(items, { ...NO_FILTERS, source: 'built' }))).toEqual(['island']);
    expect(ids(filterItems(items, { ...NO_FILTERS, format: 'obj' }))).toEqual(['crypt-brazier']);
    expect(ids(filterItems(items, { ...NO_FILTERS, usedIn: 'lantern-keeper' }))).toEqual([
      'character',
    ]);
    expect(filterItems(items, { ...NO_FILTERS, usedIn: NOT_USED }).every(i => !i.uses.length)).toBe(
      true
    );
  });

  it('sorts by use, then name', async () => {
    const { items } = await demo();
    const sorted = ids(sortItems(items, 'used'));
    expect(sorted.slice(0, 2)).toEqual(['character', 'vending-machine']);
    expect(ids(sortItems(items, 'name'))[0]).toBe('character');
  });

  it('counts tags and projects', async () => {
    const { items } = await demo();
    expect(tagCounts(items)[0]).toEqual(['props', 3]);
    expect(projectCounts(items)[0]).toEqual(['moon-island', 'Moon Island', 4]);
  });
});

describe('library lines', () => {
  it('reads the stats, versions, uses and credits of a model', async () => {
    const view = await demo();
    const stats = libraryStats(view.items);
    expect([stats.models, stats.shared, stats.updates]).toEqual([8, 2, 2]);
    const character = byId(view, 'character');
    expect(itemLine(character)).toBe('v3 · Meshy, rigged');
    expect(usedLine(character)).toBe('2 projects');
    expect(creditLine(character)).toEqual({ text: '40 saved by reuse', saved: true });
    expect(creditLine(byId(view, 'island'))).toEqual({ text: 'no credits, built', saved: false });
    expect(pictureOf(byId(view, 'island'))).toBe(`${view.dir}/island/thumbs/island_v1.webp`);
    expect(pictureOf(byId(view, 'crypt-brazier'))).toBeNull();
  });
});

describe('updateFor', () => {
  const asset = (version: number) => ({ origin: { entry: 'character', version } }) as Asset;

  it('finds an update only when the entry has a newer version', async () => {
    const view = await demo();
    expect(updateFor(view, asset(2))).toMatchObject({ from: 2, to: 3 });
    expect(updateFor(view, asset(3))).toBeNull();
    expect(updateFor(view, { origin: null } as Asset)).toBeNull();
    expect(updateFor(null, asset(1))).toBeNull();
  });
});

describe('use in project', () => {
  const website = { name: 'Moon Island', target: 'website' } as ProjectManifest;
  const game = {
    name: 'Crypt',
    target: 'game',
    game: { format: 'glb' },
  } as ProjectManifest;

  it('refuses an .obj model in a website and says what a game does with it', async () => {
    const view = await demo();
    const brazier = byId(view, 'crypt-brazier');
    expect(useBlocked(brazier, website)).toMatch(/builds its scene from .glb models/);
    expect(useBlocked(brazier, game)).toBeNull();
    expect(useNote(brazier, game)).toMatch(/Game export writes it as .glb; nothing is converted/);
    expect(useNote(byId(view, 'island'), game)).toBe(
      'The next run exports it into the game as .glb.'
    );
    expect(useBlocked(brazier, null)).toBe('Open a project first');
  });
});
