import { describe, expect, it } from 'vitest';
import type { PackItem, PackRecord } from '@/ipc/client';
import { DEFAULT_SETTINGS, newItem } from '@/ipc/mock/pack-fixtures';
import {
  budgetsLine,
  defaultName,
  filterCounts,
  formatTook,
  itemNote,
  needsYou,
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
    expect(filterCounts(items)).toEqual({ all: 6, pass: 2, look: 1, failed: 1, running: 2 });
    expect(needsYou(items).map(i => i.model.key)).toEqual(['c', 'd']);
  });

  it('reads the time left from the models done so far', () => {
    expect(timeLeft([item('a', 'queued')])).toBeNull();
    expect(timeLeft([item('a', 'pass', { tookMs: 20_000 }), item('b', 'queued')])).toBe(
      'about 20 s left'
    );
    expect(formatTook(192_000)).toBe('3:12');
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
});
