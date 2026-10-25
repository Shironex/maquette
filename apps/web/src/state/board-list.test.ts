import { describe, expect, it } from 'vitest';
import type { ModelFile } from '@/ipc/client';
import { moonIsland } from '@/ipc/mock/fixtures';
import {
  DEFAULT_PREFS,
  filterRows,
  listRow,
  nextSort,
  parsePrefs,
  shippedBytes,
  sortRows,
} from './board-list';

const assets = moonIsland().assets ?? [];
const byName = (name: string) => assets.find(a => a.name === name)!;
const models: ModelFile[] = [
  { model: 'island_v7', assetName: 'Island', version: 7, rawBytes: 4.6e6, shippedBytes: 1.2e6 },
  { model: 'vending', assetName: 'Vending machine', version: 2, rawBytes: 1e6, shippedBytes: 4e5 },
  { model: 'shop_house_v2', assetName: 'Shop house', version: 2, rawBytes: 1e6, shippedBytes: 9 },
];
const rows = assets.map(a => listRow(a, models));
const names = (list: { asset: { name: string } }[]) => list.map(r => r.asset.name);

describe('the Board list', () => {
  it('reads the shipped size of the newest model, by file then by asset', () => {
    expect(shippedBytes(byName('Island'), models)).toBe(1.2e6);
    expect(shippedBytes(byName('Vending machine'), models)).toBe(4e5);
    // An older version's build is not the newest model's.
    expect(shippedBytes(byName('Shop house'), models)).toBeNull();
    expect(shippedBytes(byName('Sakura'), models)).toBeNull();
  });

  it('says where a size comes from', () => {
    const island = rows.find(r => r.asset.id === 'island')!;
    expect(island).toMatchObject({ sizeSource: 'shipped', size: '1.2 MB', triangles: 412_000 });
    const shop = rows.find(r => r.asset.id === 'shop-house')!;
    expect(shop).toMatchObject({ sizeSource: 'source', size: '900 KB' });
    expect(rows.find(r => r.asset.id === 'sakura')).toMatchObject({ sizeSource: null });
  });

  it('filters by stage and search', () => {
    expect(names(filterRows(rows, 'blender', ''))).toEqual(['Character', 'Vending machine']);
    expect(names(filterRows(rows, 'all', '  LANT '))).toEqual(['Stone lanterns']);
    expect(names(filterRows(rows, 'all', 'character'))).toEqual(['Character']);
    expect(filterRows(rows, 'ship', '')).toEqual([]);
  });

  it('sorts by a column, rows without a value last', () => {
    const byTris = sortRows(rows, { key: 'tris', ascending: false });
    expect(names(byTris).slice(0, 2)).toEqual(['Island', 'Character']);
    const up = sortRows(rows, { key: 'tris', ascending: true });
    expect(names(up)[0]).toBe('Vending machine');
    expect(up.at(-1)?.triangles).toBeNull();
    expect(names(sortRows(rows, { key: 'name', ascending: true }))[0]).toBe('Character');
    const stages = sortRows(rows, { key: 'stage', ascending: true }).map(r => r.asset.stage);
    expect(stages[0]).toBe('concept');
    expect(stages.at(-1)).toBe('web');
    const newest = sortRows(rows, { key: 'changed', ascending: false });
    expect(names(newest)[0]).toBe('Maquette landing studio');
  });

  it('flips the same column and starts a new one in its natural direction', () => {
    expect(nextSort({ key: 'tris', ascending: false }, 'tris')).toEqual({
      key: 'tris',
      ascending: true,
    });
    expect(nextSort(DEFAULT_PREFS.sort, 'name')).toEqual({ key: 'name', ascending: true });
    expect(nextSort(DEFAULT_PREFS.sort, 'size')).toEqual({ key: 'size', ascending: false });
  });

  it('reads back only prefs it knows', () => {
    expect(parsePrefs(null)).toEqual(DEFAULT_PREFS);
    expect(parsePrefs('list')).toEqual(DEFAULT_PREFS);
    expect(
      parsePrefs({
        layout: 'list',
        sort: { key: 'tris', ascending: true },
        stage: 'web',
        query: 'x',
      })
    ).toEqual({ layout: 'list', sort: { key: 'tris', ascending: true }, stage: 'web', query: 'x' });
    expect(parsePrefs({ layout: 'grid', sort: { key: 'nope' }, stage: 'mars', query: 4 })).toEqual(
      DEFAULT_PREFS
    );
  });
});
