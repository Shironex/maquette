import { describe, expect, it } from 'vitest';
import type { PackItem, PackRecord } from '@/ipc/client';
import { DEFAULT_SETTINGS, newItem } from '@/ipc/mock/pack-fixtures';
import {
  budgetsLine,
  defaultName,
  doneLine,
  importRows,
  triangleTotals,
  filterCounts,
  formatTook,
  itemNote,
  needsYou,
  nextModel,
  packName,
  packTree,
  readme,
  timeLeft,
} from './packs';

const item = (key: string, state: PackItem['state'], extra: Partial<PackItem> = {}): PackItem => ({
  ...newItem(
    {
      key,
      name: key,
      kind: 'prop',
      entry: key,
      project: null,
      asset: null,
      version: 1,
      file: `/ws/library/${key}/models/${key}_v1.glb`,
      stats: { triangles: 9000, bytes: 1, textures: 1, maxTexture: 1024, joints: 0, animations: 0 },
      rigged: false,
      heightM: 0.92,
      mixamo: false,
      source: 'tripo',
      licence: '',
      thumbnail: null,
      refs: [],
      prep: null,
    },
    3000
  ),
  state,
  ...extra,
});

const record = (items: PackItem[]): PackRecord => ({
  version: 1,
  id: 'village-1',
  name: 'village',
  label: `${items.length} models from the library`,
  source: { from: 'entries', entries: items.map(i => i.model.key) },
  settings: { ...DEFAULT_SETTINGS, name: 'village' },
  dir: '/ws/exports/village',
  zip: '/ws/exports/village.zip',
  state: 'done',
  items,
  createdAt: '2026-10-31T14:02:00Z',
  finishedAt: '2026-10-31T14:05:12Z',
  bytes: 13_600_000,
  message: null,
  preset: 'plain-obj',
});

describe('export pack rules', () => {
  it('names a pack after its source', () => {
    expect(packName('Village props!')).toBe('village-props');
    expect(defaultName({ from: 'collection', collection: 'c' }, { collection: 'Market set' })).toBe(
      'market-set'
    );
    expect(defaultName({ from: 'project', project: 'moon-island' }, {})).toBe('moon-island');
    expect(defaultName({ from: 'entries', entries: [] }, {})).toBe('library-pack');
  });

  it('counts the check filters and what needs a person', () => {
    const items = [
      item('a', 'pass'),
      item('b', 'kept'),
      item('c', 'warn'),
      item('d', 'fail'),
      item('e', 'running'),
      item('f', 'queued'),
    ];
    expect(filterCounts(items)).toEqual({
      all: 6,
      pass: 2,
      look: 1,
      failed: 1,
      running: 1,
      waiting: 1,
    });
    expect(needsYou(items).map(i => i.model.key)).toEqual(['c', 'd']);
  });

  it('reads the time left from the models done so far', () => {
    expect(timeLeft([item('a', 'queued')])).toBeNull();
    expect(timeLeft([item('a', 'pass', { tookMs: 20_000 }), item('b', 'queued')])).toBe(
      'about 20 s left'
    );
    expect(formatTook(192_000)).toBe('3:12');
  });

  it('leaves the models still queued out of the time left, as the core records them', () => {
    const ran = Array.from({ length: 8 }, (_, n) => item(`r${n}`, 'pass', { tookMs: 5_000 }));
    const queued = Array.from({ length: 423 }, (_, n) => item(`q${n}`, 'queued', { tookMs: 0 }));
    expect(timeLeft([...ran, item('now', 'running', { tookMs: 0 }), ...queued])).toBe(
      'about 35 min left'
    );
  });

  it('reads decimated models and models left as they are apart', () => {
    const light = { stats: { ...item('a', 'pass').model.stats!, triangles: 500 } };
    const quick = (key: string, state: PackItem['state'], tookMs = 0) => {
      const it = item(key, state, { tookMs });
      return { ...it, model: { ...it.model, ...light } };
    };
    const slow = item('slow', 'pass', { tookMs: 60_000 });
    const items = [
      slow,
      quick('done', 'pass', 2_000),
      item('next', 'queued'),
      quick('later', 'queued'),
    ];
    expect(timeLeft(items)).toBe('about 1 min left');
    expect(timeLeft([quick('done', 'pass', 2_000), item('next', 'queued')])).toBe('about 2 s left');
  });

  it('still reads the time left after a stop and go on', () => {
    const again = item('again', 'queued', { tookMs: 90_000 });
    const items = [item('a', 'pass', { tookMs: 10_000 }), again, item('c', 'queued')];
    expect(timeLeft(items)).toBe('about 20 s left');
    expect(nextModel(items)).toBe(2);
  });

  it('goes on from the first model not settled', () => {
    expect(nextModel([item('a', 'pass'), item('b', 'running'), item('c', 'queued')])).toBe(2);
    expect(nextModel([item('a', 'pass'), item('b', 'fail'), item('c', 'queued')])).toBe(3);
  });

  it('says what each card waits on', () => {
    expect(itemNote(item('a', 'running'))).toBe('Decimating to 3,000');
    expect(itemNote(item('a', 'queued'))).toBe('Waiting for Blender');
  });

  it('writes the README and the tree the pack carries', () => {
    const done = record([
      item('crate', 'pass', { files: ['crate/crate.obj', 'crate/crate.mtl', 'crate/crate.png'] }),
      item('hero', 'fail'),
    ]);
    expect(budgetsLine(done.settings)).toBe('props 3,000, characters as they are, scenes 20,000');
    expect(readme(done)).toContain('Format    OBJ, MTL, PNG, Y up, metres, real size');
    expect(readme(done)).not.toContain('Mixamo');
    const mixamo = record([item('hero', 'pass', { clipsLeftOut: 2 })]);
    expect(readme(mixamo)).toContain('Mixamo clips left out');
    expect(packTree(done).map(r => r.path)).toEqual([
      'README.txt',
      'manifest.json',
      'contact-sheet.png',
      'crate/',
      'crate.obj',
      'crate.mtl',
      'crate.png',
    ]);
  });

  it('counts a warned model that shipped, and sums its triangles', () => {
    const files = (k: string) => [`${k}/${k}.obj`];
    const done = record([
      item('a', 'pass', { files: files('a'), triangles: 100 }),
      item('b', 'warn', { files: files('b'), triangles: 200, note: 'Thin parts lost at 3k tris.' }),
      item('c', 'kept', { files: files('c'), triangles: 300 }),
    ]);
    expect(doneLine(done)).toBe(
      '3 of 3 models are in the pack, 1 lost thin parts at their budget and 1 carry another warning, kept.'
    );
    expect(triangleTotals(done).after).toBe(600);
    expect(importRows(done)).toEqual([{ label: 'Blender OBJ import', opened: 3, of: 3 }]);
  });

  it('leaves out a failed model and a warning a passing zip left out', () => {
    const files = (k: string) => [`${k}/${k}.obj`];
    const failed = record([
      item('a', 'pass', { files: files('a'), triangles: 100 }),
      item('b', 'fail', { files: files('b'), triangles: 999 }),
    ]);
    expect(triangleTotals(failed).after).toBe(100);
    expect(importRows(failed)).toEqual([{ label: 'Blender OBJ import', opened: 1, of: 2 }]);
    const passing = {
      ...record([
        item('a', 'pass', { files: files('a') }),
        item('b', 'warn', { files: files('b') }),
      ]),
      message: '1 model left out of the pack.',
    };
    expect(doneLine(passing)).toBe('1 of 2 models are in the pack. 1 model left out of the pack.');
  });
});
