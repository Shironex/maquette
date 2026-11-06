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

  it('still refuses pins, pose keys and grips as not bakeable yet', async () => {
    const doc = await commands.recipeGet('frostveil', JOINED);
    const keys: RecipeLayer = {
      id: 'l-9',
      kind: 'pose',
      bone: '',
      settings: { ...doc.recipe.layers[0]!.settings, rotation: null, start: 0, end: 1 },
      keys: [{ clip: 1, frame: 4, pose: { chest: [0, -18, 0] } }],
    };
    await commands.recipeUpdate('frostveil', JOINED, {
      name: null,
      layers: [...doc.recipe.layers, keys],
      blanks: null,
      expected: doc.version,
    });
    await expect(
      commands.recipeBakeStart({ project: 'frostveil', recipe: JOINED, asset: 'clay-figure' })
    ).rejects.toMatchObject({
      code: 'not-bakeable-yet',
      message: expect.stringMatching(/pose keys/),
    });
  });
});
