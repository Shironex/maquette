import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeLayer } from '../client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { timeline } from '@/state/recipe-timeline';

const JOINED = 'slash-into-block';

beforeEach(async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  await openMockLibrary('demo');
});

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe('joined recipes in mock mode', () => {
  it('opens the demo with three clips, the block found with other keys', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    expect(doc.recipe.version).toBe(2);
    expect(doc.recipe.joined?.map(c => c.name)).toEqual([
      'Sword And Shield Block',
      'Sword And Shield Idle',
    ]);
    expect(doc.joined?.map(m => m.status)).toEqual(['hash-differs', 'found']);
    expect(doc.mixamo).toBe(true);
  });

  it('bakes joined clips, trims, time edits and the mirror over the whole timeline', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    const mirrored = await commands.recipeUpdate('frostveil', JOINED, {
      name: null,
      layers: null,
      blanks: null,
      mirror: true,
      expected: doc.version,
    });
    const record = await commands.recipeBakeStart({
      project: 'frostveil',
      recipe: JOINED,
      asset: 'clay-figure',
      expected: mirrored.version,
    });
    expect(record.state).toBe('running');
    await vi.advanceTimersByTimeAsync(10_000);
    const done = await commands.recipeBakeGet('frostveil', JOINED);
    expect(done?.result?.frames).toBe(timeline(mirrored.recipe).frames);
    expect(done?.result?.frames).not.toBe(mirrored.recipe.base.frames);
    const joinedClips = (done?.result as { joinedClips?: { name: string }[] }).joinedClips;
    expect(joinedClips?.map(c => c.name)).toEqual([
      'Sword And Shield Block',
      'Sword And Shield Idle',
    ]);
  });

  it('bakes pose keys on a joined clip and judges their joint limits', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    const keys: RecipeLayer = {
      id: 'l-9',
      kind: 'pose',
      bone: '',
      settings: { ...doc.recipe.layers[0]!.settings, rotation: null, start: 0, end: 1 },
      keys: [
        { clip: 1, frame: 4, pose: { chest: [0, -18, 0] } },
        { clip: 1, frame: 12, pose: { chest: [0, -52, 0] } },
      ],
    };
    const keyed = await commands.recipeUpdate('frostveil', JOINED, {
      name: null,
      layers: [...doc.recipe.layers, keys],
      blanks: null,
      expected: doc.version,
    });
    await commands.recipeBakeStart({
      project: 'frostveil',
      recipe: JOINED,
      asset: 'clay-figure',
      expected: keyed.version,
    });
    await vi.advanceTimersByTimeAsync(10_000);
    const done = await commands.recipeBakeGet('frostveil', JOINED);
    expect(done?.result?.joints?.judged).toBe(2);
    expect(done?.result?.joints?.hits.map(h => [h.hit.bone, h.hit.word, h.hit.limit])).toEqual([
      ['chest', 'twist', -45],
    ]);
    const line = done?.review?.lines.find(l => l.id === 'joints');
    expect(line).toMatchObject({ status: 'warn', value: 'chest twist 52 deg', max: 45 });
  });

  it('bakes a grip over joined clips and says how many bones it posed', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    const grip: RecipeLayer = {
      id: 'l-9',
      kind: 'grip',
      bone: 'rightHand',
      // Shares run along the clips as joined, so the last frame of this recipe sits near 0.6.
      settings: { ...doc.recipe.layers[0]!.settings, rotation: null, start: 0.25, end: 0.5 },
      mode: 'replace',
      pose: { rightIndexProximal: [62, 0, 0], rightHand: [4, 0, -8] },
      from: { entry: 'longsword-two-hands', revision: 2 },
    };
    const held = await commands.recipeUpdate('frostveil', JOINED, {
      name: null,
      layers: [...doc.recipe.layers, grip],
      blanks: null,
      expected: doc.version,
    });
    await commands.recipeBakeStart({
      project: 'frostveil',
      recipe: JOINED,
      asset: 'clay-figure',
      expected: held.version,
    });
    await vi.advanceTimersByTimeAsync(10_000);
    const done = await commands.recipeBakeGet('frostveil', JOINED);
    const frames = timeline(held.recipe).frames;
    const [laid] = done?.result?.grips ?? [];
    expect(laid).toMatchObject({
      layer: 'l-9',
      bone: 'rightHand',
      entry: 'longsword-two-hands',
      revision: 2,
      bones: 2,
      posed: 2,
      missing: [],
    });
    expect(laid?.frames?.[0]).toBeGreaterThan(0);
    expect(laid?.frames?.[1]).toBeLessThan(frames);
    const line = done?.review?.lines.find(l => l.id === 'grip-l-9');
    expect(line).toMatchObject({ label: 'Grip', status: 'pass', value: '2 of 2 bones' });
  });
});
