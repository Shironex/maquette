import { describe, expect, it } from 'vitest';
import type {
  LibraryAdded,
  LibraryUpdated,
  LibraryUsed,
  LibraryView,
  ProjectManifest,
  UpdatePlan,
} from '../client';
import { createBackend } from './backend';

function backend(library: string | null = 'demo') {
  return createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, library, emit: () => {} });
}

describe('the mock library', () => {
  it('opens empty without the demo', async () => {
    const view = (await backend(null)('library_get')) as LibraryView;
    expect(view.items).toEqual([]);
    expect(view.dir).toMatch(/\/library$/);
  });

  it('knows which projects use each entry and which wait for an update', async () => {
    const view = (await backend()('library_get')) as LibraryView;
    const lantern = view.items.find(i => i.entry.id === 'crystal-lantern')!;
    expect(lantern.uses.map(u => [u.project, u.version, u.update])).toEqual([
      ['frostveil', 2, false],
      ['frost-cathedral', 1, true],
    ]);
    const character = view.items.find(i => i.entry.id === 'elsheria')!;
    expect(character.uses.find(u => u.project === 'frostveil')?.update).toBe(true);
  });

  it('uses an entry in a project as a new asset that records its origin', async () => {
    const handle = backend();
    const used = (await handle('library_use', {
      entry: 'guardian-statue',
      version: null,
      project: 'frost-cathedral',
    })) as LibraryUsed;
    expect(used.asset.origin).toEqual({ entry: 'guardian-statue', version: 1 });
    expect(used.asset.versions).toHaveLength(1);
    const again = (await handle('library_use', {
      entry: 'guardian-statue',
      version: null,
      project: 'frost-cathedral',
    })) as LibraryUsed;
    expect(again.asset.name).toBe('Guardian statue 2');
  });

  it('refuses an .obj model in a website project', async () => {
    await expect(
      backend()('library_use', { entry: 'crypt-brazier', version: null, project: 'frostveil' })
    ).rejects.toMatchObject({ code: 'invalid' });
  });

  it('plans and makes an update that drops a decimate the new model no longer needs', async () => {
    const handle = backend();
    const plan = (await handle('library_update_plan', {
      project: 'frostveil',
      asset: 'elsheria',
    })) as UpdatePlan;
    expect([plan.from, plan.to]).toEqual([2, 3]);
    expect(plan.dropped.map(d => d.what)).toEqual(['Decimate to 60k']);
    expect(plan.kept).toContain('Origin to bottom centre');
    const done = (await handle('library_update', {
      project: 'frostveil',
      asset: 'elsheria',
    })) as LibraryUpdated;
    expect(done.asset.origin).toEqual({ entry: 'elsheria', version: 3 });
    expect(done.asset.versions.at(-1)?.version).toBe(plan.nextVersion);
  });

  it('adds a model once and records the origin on the asset', async () => {
    const handle = backend(null);
    const added = (await handle('library_add', {
      request: { project: 'frostveil', asset: 'observatory-arch', name: null, tags: ['Props'] },
    })) as LibraryAdded;
    expect([added.added, added.version, added.entry.tags]).toEqual([true, 1, ['props']]);
    const house = (added.project as ProjectManifest).assets?.find(a => a.id === 'observatory-arch');
    expect(house?.origin).toEqual({ entry: added.entry.id, version: 1 });
    const again = (await handle('library_add', {
      request: { project: 'frostveil', asset: 'observatory-arch', name: null, tags: null },
    })) as LibraryAdded;
    expect(again.added).toBe(false);
  });
});
