import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeBase, type RecipeDoc } from '../client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { timeline } from '@/state/recipe-timeline';
import { seamLines, seamsOf, windowsOf } from './recipe-seams';

const JOINED = 'slash-into-block';

const clip = (name: string, frames: number): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
});

/** A recipe on two 40 frame clips with a `blend` frame window, as the core's tests have it. */
function joined(blend: number): RecipeDoc {
  const recipe = {
    version: 2,
    name: 'Two clips',
    revision: 1,
    created: '',
    edited: '',
    base: clip('Slash', 40),
    joined: [{ ...clip('Thrust', 40), blend: { frames: blend } }],
    blanks: [],
    layers: [],
    from: null,
  };
  return { recipe } as unknown as RecipeDoc;
}

describe('seam windows in mock mode, as the core lays them', () => {
  it('judges a blend from its first frame to the frame after it, and a cut on its one step', () => {
    const blended = joined(8);
    expect(windowsOf(blended, timeline(blended.recipe)).map(w => [w.first, w.last])).toEqual([
      [32, 40],
    ]);
    const cut = joined(0);
    expect(windowsOf(cut, timeline(cut.recipe)).map(w => [w.first, w.last])).toEqual([[40, 40]]);
  });

  it('judges a hold over where it eases in and out', () => {
    const doc = joined(8);
    const one = { ...doc, recipe: { ...doc.recipe, joined: [], base: clip('Charge', 60) } };
    one.recipe.time = [
      { kind: 'hold-over', clip: 0, at: 20, frames: 10, easeIn: 3, easeOut: 2 },
    ] as RecipeDoc['recipe']['time'];
    const found = windowsOf(one, timeline(one.recipe)).map(w => [w.kind, w.first, w.last]);
    expect(found).toEqual([
      ['hold-in', 21, 23],
      ['hold-out', 29, 30],
    ]);
  });
});

describe('near seams and unmeasured motion in mock mode', () => {
  it('lets two near joins each pop on their own', () => {
    const doc = joined(2);
    doc.recipe.joined = [
      { ...clip('Thrust', 6), blend: { frames: 2 } },
      { ...clip('Block', 12), blend: { frames: 2 } },
    ] as RecipeDoc['recipe']['joined'];
    const joins = seamsOf(doc).filter(s => s.kind === 'join');
    expect(joins).toHaveLength(2);
    // Each sits in the other's six frames around: held to the near seam's
    // jump, neither would pop.
    expect(joins[1]!.first - joins[0]!.last).toBeLessThanOrEqual(6);
    expect(joins.map(s => s.hipsAroundCm)).toEqual([1.2, 1.2]);
    expect(joins.every(s => s.pops && s.measured)).toBe(true);
  });

  it('reads seams the bake could not measure as not measured', () => {
    const seams = seamsOf(joined(2)).map(s => ({ ...s, measured: false, pops: false }));
    const { pops } = seamLines({ seams } as unknown as Parameters<typeof seamLines>[0]);
    expect(pops[0]).toMatchObject({ status: 'not-measured', value: 'not measured' });
  });

  it('never reads pass while one of the seams was not measured, but a pop still warns', () => {
    const [seam] = seamsOf(joined(20));
    const passing = { ...seam!, pops: false, hipsOverCm: 0.5 };
    const unmeasured = { ...seam!, measured: false, pops: false, hipsOverCm: 0 };
    const lines = (seams: unknown[]) =>
      seamLines({ seams } as unknown as Parameters<typeof seamLines>[0]).pops;
    expect(lines([passing])[0]).toMatchObject({ status: 'pass' });
    expect(lines([passing, unmeasured])[0]).toMatchObject({
      status: 'not-measured',
      note: expect.stringMatching(/^1 of 2 seams were not measured/),
    });
    const popping = { ...seam!, pops: true, hipsOverCm: 5 };
    expect(lines([unmeasured, popping])[0]).toMatchObject({
      status: 'warn',
      note: expect.stringMatching(/ 1 not measured\.$/),
    });
  });
});

describe('seam checks in a mock bake', () => {
  beforeEach(async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await openMockLibrary('demo');
  });

  afterEach(() => {
    vi.useRealTimers();
    clearMocks();
  });

  it('pops where clips meet with a short blend, sheets the seam and offers a wider blend', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    await commands.recipeBakeStart({
      project: 'frostveil',
      recipe: JOINED,
      asset: 'clay-figure',
      expected: doc.version,
    });
    await vi.advanceTimersByTimeAsync(10_000);
    const done = await commands.recipeBakeGet('frostveil', JOINED);
    const review = done?.review;
    const seam = review?.lines.find(l => l.id === 'seam');
    expect(seam).toMatchObject({ label: 'Seam pop', status: 'warn', limit: '2 cm' });
    expect(review?.lines[0]?.id).toBe('seam');
    const fix = review?.fixes?.find(f => f.line === 'seam');
    expect(fix?.edit).toMatchObject({ kind: 'blend', clip: 1 });
    expect(fix?.title).toMatch(/^Widen the blend to \d+ f$/);
    const sheet = done?.result?.sheet ?? [];
    expect(sheet.some(f => f.kind === 'seam' && f.warn && f.label === 'seam 1')).toBe(true);
    expect(sheet.length).toBeLessThanOrEqual(8);
  });
});
