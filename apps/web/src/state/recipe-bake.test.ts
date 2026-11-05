import { describe, expect, it } from 'vitest';
import { RESULT, record } from '@/test/recipe-bake-fixture';
import {
  RECIPE_CHANGED,
  applyBlocked,
  progressOf,
  resultLines,
  resultSummary,
  staleOf,
} from './recipe-bake';

describe('a recipe bake', () => {
  it('says why Apply waits, in the order a person fixes it', () => {
    const ready = { open: 0, layers: 2, model: 'elsheria', running: false };
    expect(applyBlocked(ready)).toBeNull();
    expect(applyBlocked({ ...ready, running: true })).toBe('This recipe is baking');
    expect(applyBlocked({ ...ready, layers: 0, open: 1 })).toBe('Add a change first');
    expect(applyBlocked({ ...ready, open: 1 })).toBe('Fill the open blank first');
    expect(applyBlocked({ ...ready, open: 2 })).toBe('Fill the 2 open blanks first');
    expect(applyBlocked({ ...ready, model: null })).toMatch(/No rigged model/);
  });

  it('reads as out of date once the recipe or the model moves on', () => {
    const model = { name: 'Elsheria', version: 2 };
    expect(staleOf(record(), { recipeVersion: 'v1', model })).toBeNull();
    expect(staleOf(record(), { recipeVersion: 'v2', model })).toBe(RECIPE_CHANGED);
    expect(staleOf(record(), { recipeVersion: 'v1', model: { ...model, version: 3 } })).toBe(
      'Elsheria is at v3 now, and this bake went onto v2. Bake it again.'
    );
    // What the view does not know, the core's last read says.
    expect(staleOf(record({ stale: 'Gone.' }), { recipeVersion: 'v1' })).toBe('Gone.');
    expect(staleOf(record({ stale: 'Gone.' }), { recipeVersion: 'v1', model })).toBeNull();
    expect(staleOf(record({ state: 'running', result: null }), { recipeVersion: 'v9' })).toBeNull();
  });

  it('reads its progress as a label and a share', () => {
    expect(progressOf(record({ state: 'running', result: null }))).toEqual({
      label: 'Starting Blender...',
      share: 0,
    });
    const going = record({
      state: 'running',
      progress: { label: 'Solving frame 12 of 31', done: 12, total: 31 },
    });
    expect(progressOf(going).label).toBe('Solving frame 12 of 31');
    expect(progressOf(going).share).toBeCloseTo(12 / 31);
  });

  it('lists each reach, the feet and the rig, marking what needs a look', () => {
    const lines = resultLines(RESULT);
    expect(lines.map(l => [l.label, l.value, l.bad])).toEqual([
      ['Reach rightHand', '0.4 cm from its target', false],
      ['Reach leftHand', '3.2 cm from its target', true],
      ['Foot slide', '0.8 cm', false],
      ['Rig stretch', '3.4 mm', false],
    ]);
    expect(lines[1]?.note).toBe('The joint bends the other way from frame 18.');
    expect(lines[3]?.note).toBe('The base clip: 2.9 mm.');
    const unmeasured = resultLines({ ...RESULT, stretchMm: null, stretchNote: 'No skin.' });
    expect(unmeasured[3]).toMatchObject({ value: 'not measured', note: 'No skin.' });
    expect(resultSummary(RESULT)).toBe('reach_out · 31 f · onto Elsheria v2 · 2.6 s');
  });
});
