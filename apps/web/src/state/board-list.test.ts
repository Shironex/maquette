import { describe, expect, it } from 'vitest';
import type { ModelFile } from '@/ipc/client';
import { frostveilProject } from '@/ipc/mock/fixtures';
import {
  DEFAULT_PREFS,
  filterRows,
  listRow,
  nextSort,
  parsePrefs,
  shippedBytes,
  sortRows,
} from './board-list';

const assets = frostveilProject().assets ?? [];
const byName = (name: string) => assets.find(a => a.name === name)!;
const models: ModelFile[] = [
  {
    model: 'veil_platform_v7',
    assetName: 'Veil platform',
    version: 7,
    rawBytes: 4.6e6,
    shippedBytes: 1.2e6,
  },
  {
    model: 'crystal_lantern',
    assetName: 'Crystal lantern',
    version: 2,
    rawBytes: 1e6,
    shippedBytes: 4e5,
  },
  {
    model: 'observatory_arch_v2',
    assetName: 'Observatory arch',
    version: 2,
    rawBytes: 1e6,
    shippedBytes: 9,
  },
];
const rows = assets.map(a => listRow(a, models));
const names = (list: { asset: { name: string } }[]) => list.map(r => r.asset.name);

describe('the Board list', () => {
  it('reads the shipped size of the newest model, by file then by asset', () => {
    expect(shippedBytes(byName('Veil platform'), models)).toBe(1.2e6);
    expect(shippedBytes(byName('Crystal lantern'), models)).toBe(4e5);
    // An older version's build is not the newest model's.
    expect(shippedBytes(byName('Observatory arch'), models)).toBeNull();
    expect(shippedBytes(byName('Veil gate'), models)).toBeNull();
  });

  it('says where a size comes from', () => {
    const island = rows.find(r => r.asset.id === 'veil-platform')!;
    expect(island).toMatchObject({ sizeSource: 'shipped', size: '1.2 MB', triangles: 412_000 });
    const shop = rows.find(r => r.asset.id === 'observatory-arch')!;
    expect(shop).toMatchObject({ sizeSource: 'source', size: '900 KB' });
    expect(rows.find(r => r.asset.id === 'veil-gate')).toMatchObject({ sizeSource: null });
  });

  it('filters by stage and search', () => {
    expect(names(filterRows(rows, 'blender', ''))).toEqual(['Elsheria', 'Crystal lantern']);
    expect(names(filterRows(rows, 'all', '  GUARD '))).toEqual(['Guardian statues']);
    expect(names(filterRows(rows, 'all', 'character'))).toEqual(['Elsheria']);
    expect(names(filterRows(rows, 'ship', ''))).toEqual(['Veil platform']);
  });

  it('sorts by a column, rows without a value last', () => {
    const byTris = sortRows(rows, { key: 'tris', ascending: false });
    expect(names(byTris).slice(0, 2)).toEqual(['Veil platform', 'Elsheria']);
    const up = sortRows(rows, { key: 'tris', ascending: true });
    expect(names(up)[0]).toBe('Crystal lantern');
    expect(up.at(-1)?.triangles).toBeNull();
    expect(names(sortRows(rows, { key: 'name', ascending: true }))[0]).toBe('Academy landing');
    const stages = sortRows(rows, { key: 'stage', ascending: true }).map(r => r.asset.stage);
    expect(stages[0]).toBe('concept');
    expect(stages.at(-1)).toBe('ship');
    const newest = sortRows(rows, { key: 'changed', ascending: false });
    expect(names(newest)[0]).toBe('Academy landing');
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
