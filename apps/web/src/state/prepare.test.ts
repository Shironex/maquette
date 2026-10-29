import { describe, expect, it } from 'vitest';
import type { FixupReport } from '@/ipc/client';
import {
  EMPTY_PREP,
  hasNoPlan,
  isEmptyPrep,
  skipsBake,
  withSkipBake,
  defaultFixup,
  describeFixup,
  describeSize,
  reportRows,
  shortCount,
  startSize,
  withFixup,
  withoutFixup,
  workspaceDefaults,
} from './prepare';

describe('real size and fix-up words', () => {
  it('reads the way the core writes them', () => {
    expect(describeSize({ axis: 'height', value: 45, unit: 'cm' })).toBe('45 cm high');
    expect(describeSize({ axis: 'longest', value: 1.25, unit: 'm' })).toBe(
      '1.25 m on its longest side'
    );
    expect(describeFixup({ op: 'decimate', triangles: 60_000 })).toBe('Decimate to 60k');
    expect(describeFixup({ op: 'decimate', triangles: 4999 })).toBe('Decimate to 5k');
    expect(describeFixup({ op: 'remove-loose', minShare: 0.02 })).toBe(
      'Remove loose parts under 2%'
    );
    expect(describeFixup({ op: 'merge', byMaterial: true })).toBe('Merge objects by material');
    expect(describeFixup({ op: 'rotate', x: -24.5, y: 0, z: 90 })).toBe('Rotate x -24.5°, z 90°');
    expect(describeFixup({ op: 'rotate', x: 0, y: 0, z: 0 })).toBe('Rotate (no turn)');
    expect(shortCount(1_250_000)).toBe('1.3M');
    expect(shortCount(812)).toBe('812');
  });
});

describe('editing the fix-ups', () => {
  it('keeps one of a kind, in the order they apply', () => {
    let prep = withFixup(EMPTY_PREP, { op: 'origin-bottom' });
    prep = withFixup(prep, { op: 'decimate', triangles: 60_000 });
    prep = withFixup(prep, { op: 'decimate', triangles: 5000 });
    expect(withFixup(prep, defaultFixup('rotate', workspaceDefaults(null))).fixups[0]).toEqual({
      op: 'rotate',
      x: 0,
      y: 0,
      z: 0,
    });
    expect(prep.fixups).toEqual([{ op: 'decimate', triangles: 5000 }, { op: 'origin-bottom' }]);
    expect(withoutFixup(prep, 'decimate').fixups).toEqual([{ op: 'origin-bottom' }]);
  });

  it('starts new ones from the workspace settings', () => {
    const d = workspaceDefaults({
      budgets: { trisPerModel: 40_000 },
      compression: { textureSize: '1K' },
    } as never);
    expect(defaultFixup('decimate', d)).toEqual({ op: 'decimate', triangles: 40_000 });
    expect(defaultFixup('shrink-textures', d)).toEqual({ op: 'shrink-textures', maxSize: 1024 });
    expect(startSize([0.5, 0.7, 0.5])).toEqual({ axis: 'height', value: 70, unit: 'cm' });
    expect(startSize([11.7, 8, 7.6])).toEqual({ axis: 'height', value: 8, unit: 'm' });
  });

  it('lists a preview row by row and marks what changed', () => {
    const n = { triangles: 100, objects: 2, maxTexture: 1024, min: null, max: null };
    const report = {
      before: n,
      after: { ...n, objects: 1 },
      bytesBefore: 2e6,
      bytesAfter: 2e6,
    } as FixupReport;
    const rows = reportRows(report);
    expect(rows.find(r => r.label === 'Objects')).toMatchObject({
      before: '2',
      after: '1',
      changed: true,
    });
    expect(rows.find(r => r.label === 'Triangles')?.changed).toBe(false);
  });
});

describe('skipping the bake', () => {
  it('bakes unless someone chose, except an Inbox model', () => {
    expect(skipsBake(EMPTY_PREP)).toBe(false);
    expect(skipsBake(EMPTY_PREP, true)).toBe(true);
    expect(skipsBake(withSkipBake(EMPTY_PREP, false), true)).toBe(false);
  });

  it('keeps an entry that only skips the bake, though it has no size or fix-ups', () => {
    const skip = withSkipBake(EMPTY_PREP, true);
    expect(hasNoPlan(skip)).toBe(true);
    expect(isEmptyPrep(skip)).toBe(false);
    expect(isEmptyPrep(EMPTY_PREP)).toBe(true);
  });
});
