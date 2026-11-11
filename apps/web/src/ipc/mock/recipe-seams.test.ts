import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeBase, type RecipeDoc } from '../client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { timeline } from '@/state/recipe-timeline';
import { windowsOf } from './recipe-seams';

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
