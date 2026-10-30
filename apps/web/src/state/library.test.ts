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
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'guardian' }))).toEqual([
      'guardian-statue',
    ]);
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'cathedral' }))).toEqual([
      'crystal-lantern',
    ]);
    expect(ids(filterItems(items, { ...NO_FILTERS, query: 'lights props' }))).toEqual([
      'crystal-lantern',
    ]);
  });

  it('filters by tag, source, format and the project that uses it', async () => {
    const { items } = await demo();
    expect(ids(filterItems(items, { ...NO_FILTERS, tag: 'statues' }))).toEqual([
      'guardian-statue',
      'veil-gate',
    ]);
    expect(ids(filterItems(items, { ...NO_FILTERS, source: 'built' }))).toEqual(['veil-platform']);
    expect(ids(filterItems(items, { ...NO_FILTERS, format: 'obj' }))).toEqual(['crypt-brazier']);
    expect(ids(filterItems(items, { ...NO_FILTERS, usedIn: 'elsheria-showcase' }))).toEqual([
      'elsheria',
    ]);
    expect(filterItems(items, { ...NO_FILTERS, usedIn: NOT_USED }).every(i => !i.uses.length)).toBe(
      true
    );
  });

  it('sorts by use, then name', async () => {
    const { items } = await demo();
    const sorted = ids(sortItems(items, 'used'));
    expect(sorted.slice(0, 2)).toEqual(['crystal-lantern', 'elsheria']);
    expect(ids(sortItems(items, 'name'))[0]).toBe('crypt-brazier');
  });

  it('counts tags and projects', async () => {
    const { items } = await demo();
    expect(tagCounts(items)[0]).toEqual(['props', 3]);
    expect(projectCounts(items)[0]).toEqual(['frostveil', 'Frostveil', 4]);
  });
});

describe('library lines', () => {
  it('reads the stats, versions, uses and credits of a model', async () => {
    const view = await demo();
    const stats = libraryStats(view.items);
    expect([stats.models, stats.shared, stats.updates]).toEqual([8, 2, 2]);
    const character = byId(view, 'elsheria');
    expect(itemLine(character)).toBe('v3 · Meshy, rigged · 35k tris');
    expect(usedLine(character)).toBe('2 projects');
    expect(creditLine(character)).toEqual({ text: '40 saved by reuse', saved: true });
    expect(creditLine(byId(view, 'veil-platform'))).toEqual({
      text: 'no credits, built',
      saved: false,
    });
    expect(pictureOf(byId(view, 'veil-platform'))).toBe(
      `${view.dir}/veil-platform/thumbs/veil_platform_v1.webp`
    );
    const brazier = byId(view, 'crypt-brazier');
    expect(pictureOf(brazier)).toBe(`${view.dir}/crypt-brazier/thumbs/crypt_brazier_v1.webp`);
    const bare = brazier.entry.versions.map(v => ({ ...v, thumbnail: null }));
    expect(pictureOf({ ...brazier, entry: { ...brazier.entry, versions: bare } })).toBeNull();
  });
});

describe('updateFor', () => {
  const asset = (version: number) => ({ origin: { entry: 'elsheria', version } }) as Asset;

  it('finds an update only when the entry has a newer version', async () => {
    const view = await demo();
    expect(updateFor(view, asset(2))).toMatchObject({ from: 2, to: 3 });
    expect(updateFor(view, asset(3))).toBeNull();
    expect(updateFor(view, { origin: null } as Asset)).toBeNull();
    expect(updateFor(null, asset(1))).toBeNull();
  });
});

describe('use in project', () => {
  const website = { name: 'Frostveil', target: 'website' } as ProjectManifest;
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
    expect(useNote(byId(view, 'veil-platform'), game)).toBe(
      'The next run exports it into the game as .glb.'
    );
    expect(useBlocked(brazier, null)).toBe('Open a project first');
  });
});
