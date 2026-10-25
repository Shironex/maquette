import { describe, expect, it } from 'vitest';
import type { BuildOutcome } from '@/ipc/client';
import { builtChip, fileName, nameFromScript, outcomeNotice, wantsRebuild } from './built';

describe('built assets', () => {
  it('names an asset after its script as the app does', () => {
    expect(nameFromScript('/Users/maker/room/scripts/newtons_cradle.py')).toBe('Newtons cradle');
    expect(nameFromScript('C:\\room\\room-shell.py')).toBe('Room shell');
    expect(nameFromScript('')).toBe('');
    expect(fileName('built/laptop.py')).toBe('laptop.py');
  });

  it('says more than the asset status only when the script asks for a build', () => {
    expect(builtChip('current')).toBeUndefined();
    expect(builtChip('stale')?.label).toBe('Script changed');
    expect(builtChip('building')?.state).toBe('running');
    expect(builtChip('failed')?.state).toBe('failed');
    expect(wantsRebuild('stale')).toBe(true);
    expect(wantsRebuild('current')).toBe(false);
    expect(wantsRebuild('building')).toBe(false);
  });

  it('tells a failed build by its error', () => {
    const outcome = {
      result: 'failed',
      version: null,
      record: { error: 'NameError: boom', warnings: [], objects: [], materials: [] },
      log: ['Traceback'],
    } as unknown as BuildOutcome;
    expect(outcomeNotice('Laptop', outcome)).toEqual({
      status: 'failed',
      title: 'Laptop did not build',
      detail: 'NameError: boom',
    });
    const built = {
      ...outcome,
      result: 'built',
      version: { version: 2 },
      record: { error: null, warnings: [], objects: ['laptop', 'lid'], materials: ['screen'] },
    } as unknown as BuildOutcome;
    expect(outcomeNotice('Laptop', built).title).toBe('Built Laptop v2');
    expect(outcomeNotice('Laptop', built).detail).toBe('2 objects, 1 materials.');
  });
});
