import { describe, expect, it } from 'vitest';
import type {
  LibraryAdded,
  LibraryCollection,
  LibraryEntry,
  LibraryMerged,
  LibraryRestored,
  LibraryView,
  LookAlikes,
} from '../client';
import { createBackend } from './backend';

function backend(library: string | null = 'scale') {
  return createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, library, emit: () => {} });
}

const view = async (handle: ReturnType<typeof backend>) =>
  (await handle('library_get')) as LibraryView;
const ids = (v: LibraryView) => v.items.map(i => i.entry.id);

describe('the mock library at scale', () => {
  it('opens with 214 models, three collections and Hollow Vale to add', async () => {
    const handle = backend();
    const v = await view(handle);
    expect(v.items).toHaveLength(214);
    expect(v.collections.map(c => [c.name, c.entries.length])).toEqual([
      ['Village props', 40],
      ['Forest kit', 30],
      ['Market set', 18],
    ]);
    const projects = (await handle('project_list')) as { id: string }[];
    expect(projects.map(p => p.id)).toContain('hollow-vale');
  });

  it('puts tags on many models and takes them off', async () => {
    const handle = backend();
    const tagged = (await handle('library_tag', {
      request: {
        entries: ['farm-table', 'writing-desk'],
        add: [' Rustic '],
        remove: ['furniture'],
      },
    })) as LibraryEntry[];
    expect(tagged.map(e => e.tags)).toEqual([['rustic'], ['rustic']]);
  });

  it('makes, fills and empties a collection, and refuses a name twice', async () => {
    const handle = backend();
    let sets = (await handle('library_collection', {
      change: { op: 'create', name: 'Lights', entries: ['street-lamp'] },
    })) as LibraryCollection[];
    expect(sets.find(c => c.id === 'lights')?.entries).toEqual(['street-lamp']);
    sets = (await handle('library_collection', {
      change: { op: 'add', collection: 'lights', entries: ['street-lamp', 'stone-lantern'] },
    })) as LibraryCollection[];
    expect(sets.find(c => c.id === 'lights')?.entries).toEqual(['street-lamp', 'stone-lantern']);
    await expect(
      handle('library_collection', { change: { op: 'create', name: 'lights', entries: [] } })
    ).rejects.toMatchObject({ code: 'invalid' });
  });

  it('removes models and restores them, leaving the projects alone', async () => {
    const handle = backend();
    await handle('library_remove', { entries: ['crystal-lantern', 'farm-table'] });
    expect(ids(await view(handle))).not.toContain('crystal-lantern');
    const back = (await handle('library_restore', {
      entries: ['crystal-lantern', 'farm-table'],
    })) as LibraryRestored;
    expect(back.entries).toHaveLength(2);
    const lantern = (await view(handle)).items.find(i => i.entry.id === 'crystal-lantern')!;
    // The copies in projects never moved, so they show again as its uses.
    expect(lantern.uses.map(u => u.project)).toEqual(['frostveil', 'frost-cathedral']);
  });

  it('finds look-alike and same name pairs, and hides a decided one', async () => {
    const handle = backend();
    const report = (await handle('library_look_alikes')) as LookAlikes;
    expect(report.pairs.map(p => [p.kind, p.a, p.b])).toEqual([
      ['same-look', 'street-lamp', 'street-lamp-2'],
      ['same-look', 'bee-hive', 'hive-box'],
      ['same-name', 'stone-well', 'stone-well-2'],
    ]);
    expect(report.unseen).toEqual(['spider-web', 'oak-trellis']);
    expect(report.merges.map(m => m.merged)).toEqual(['paper-lanterns-2']);
    await handle('library_decide', { a: 'hive-box', b: 'bee-hive', verdict: 'keep-both' });
    await handle('library_look_settings', {
      settings: { threshold: 0.95, checkOnAdd: true, copyTags: true },
    });
    const after = (await handle('library_look_alikes')) as LookAlikes;
    expect(after.pairs.map(p => p.b)).toEqual(['street-lamp-2', 'stone-well-2']);
  });

  it('merges a look-alike as versions and undoes it', async () => {
    const handle = backend();
    const merged = (await handle('library_merge', {
      request: { keep: 'street-lamp', merge: 'street-lamp-2', copyTags: true },
    })) as LibraryMerged;
    expect(merged.entry.versions.length).toBeGreaterThan(1);
    expect(ids(await view(handle))).not.toContain('street-lamp-2');
    await handle('library_restore', { entries: ['street-lamp-2'] });
    const v = await view(handle);
    expect(v.items.find(i => i.entry.id === 'street-lamp')?.entry.versions).toHaveLength(1);
    expect(ids(v)).toContain('street-lamp-2');
  });

  it('adds a model as a version of another entry', async () => {
    const handle = backend();
    const added = (await handle('library_add', {
      request: {
        project: 'hollow-vale',
        asset: 'street-lamp',
        name: null,
        tags: null,
        into: 'street-lamp',
      },
    })) as LibraryAdded;
    expect(added.entry.id).toBe('street-lamp');
    expect(added.version).toBe(2);
  });
});
