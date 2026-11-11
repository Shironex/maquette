import { describe, expect, it, vi } from 'vitest';
import type { ProjectManifest, ReplaceDone, ReplaceReport, ShipSwap, UndoDone } from '../client';
import { createBackend } from './backend';

function backend(built: string, emit = vi.fn()) {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, built, emit });
  return { handle, emit };
}

const pair = { project: 'frostveil', asset: 'crystal-lantern', built: 'lantern-rebuilt' };

describe('a built asset in the place of a model, in mock mode', () => {
  it('seeds a swap without announcing it, since nothing listens yet', async () => {
    const { handle, emit } = backend('swapped');
    expect(emit).not.toHaveBeenCalled();
    const swaps = (await handle('built_swaps', { project: 'frostveil' })) as ShipSwap[];
    expect(swaps.map(s => [s.asset, s.oldVersion, s.newVersion])).toEqual([
      ['crystal-lantern', 2, 3],
    ]);
  });

  it('plans without changing anything, with the scan settings an accept leaves', async () => {
    const { handle, emit } = backend('swap');
    const report = (await handle('built_replace_plan', pair)) as ReplaceReport;
    expect(report.old.file).toBe('models/crystal_lantern_v2.glb');
    expect(report.new.file).toBe('models/crystal_lantern_v3.glb');
    expect(report.settings.map(s => [s.what, s.action])).toEqual([
      ['Real size, 1.8 m high', 'carried'],
      ['Decimate to 8k', 'reset'],
      ['Skips the bake', 'reset'],
      ['Pads texture edges', 'reset'],
      ['Layout placement: position 3.6, 0, 1.2, turn 0, -60, 0, scale 0.8', 'kept'],
    ]);
    expect(report.warnings).toEqual([]);
    expect(emit).not.toHaveBeenCalled();
    expect(await handle('built_swaps', { project: 'frostveil' })).toEqual([]);
  });

  it('replaces, tells the windows, and undoes back to the same project', async () => {
    const { handle, emit } = backend('swap');
    const ids = (p: ProjectManifest) => (p.assets ?? []).map(a => a.id);
    const before = (await handle('built_replace_plan', pair)) as ReplaceReport;

    const done = (await handle('built_replace', pair)) as ReplaceDone;
    expect(ids(done.project)).not.toContain('lantern-rebuilt');
    expect(done.report.trash).toMatch(/^lantern-rebuilt-\d+$/);
    expect(emit).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'built', project: 'frostveil' })
    );

    const undone = (await handle('built_replace_undo', {
      project: 'frostveil',
      asset: 'crystal-lantern',
    })) as UndoDone;
    expect(undone.report.takenOff.map(v => v.version)).toEqual([3]);
    expect(undone.report.restores).toBe('Lantern rebuilt');
    expect(ids(undone.project)).toEqual([...ids(done.project), 'lantern-rebuilt']);
    const lantern = (undone.project.assets ?? []).find(a => a.id === 'crystal-lantern');
    expect(lantern?.versions.map(v => v.version)).toEqual([1, 2]);
    // The same pair plans the same again: its settings are back.
    expect(((await handle('built_replace_plan', pair)) as ReplaceReport).settings).toEqual(
      before.settings
    );
  });

  it('refuses what the app refuses', async () => {
    const { handle } = backend('swap');
    const plan = (args: Record<string, string>) =>
      handle('built_replace_plan', { project: 'frostveil', ...args });
    await expect(plan({ asset: 'laptop', built: 'laptop' })).rejects.toMatchObject({
      message: expect.stringContaining('Pick two assets'),
    });
    await expect(plan({ asset: 'laptop', built: 'lantern-rebuilt' })).rejects.toMatchObject({
      message: expect.stringContaining('Laptop is already a built asset'),
    });
    await expect(plan({ asset: 'crystal-lantern', built: 'elsheria' })).rejects.toMatchObject({
      message: expect.stringContaining('Elsheria is not a built asset'),
    });
    await expect(plan({ asset: 'crystal-lantern', built: 'newtons-cradle' })).rejects.toMatchObject(
      {
        message: expect.stringContaining('has a script that changed since its last build'),
      }
    );
  });
});
