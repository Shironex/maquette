import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeFile, RecipeLayer } from '../client';
import { layer, motionVersionOf, newRecipe } from './recipe-files';
import { checkSize, followClips, usesV2, v2Problem, V2_LIMITS, withVersion } from './recipe-v2';

const clip = (name: string, frames = 60): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: `${name}-hash`,
  mixamo: false,
});

const poseKeys = (id: string, keys: RecipeLayer['keys']): RecipeLayer => ({
  ...layer(id, 'pose', '', {}),
  keys,
});

/** A recipe on A, B and C with a key on B and C and a freeze on B. */
function onThree(): RecipeFile {
  return withVersion({
    ...newRecipe('Three', clip('A')),
    joined: [clip('B'), clip('C')],
    layers: [
      poseKeys('k', [
        { clip: 1, frame: 10, pose: { chest: [0, 20, 0] } },
        { clip: 2, frame: 30, pose: { head: [5, 0, 0] } },
      ]),
    ],
    time: [{ kind: 'freeze', clip: 1, at: 5, frames: 4 }],
  });
}

describe('the mock recipe file version 2', () => {
  it('writes the base without a blend and pose keys adding unless told, as the core does', () => {
    const recipe = withVersion({
      ...newRecipe('Keys', { ...clip('A'), blend: { frames: 4 } }),
      layers: [poseKeys('k', [{ clip: 0, frame: 9, pose: { chest: [0, 1, 0] } }])],
    });
    expect(recipe.base.blend).toBeUndefined();
    expect(recipe.layers[0]?.mode).toBe('add');
    expect(recipe.layers[0]?.settings.start).toBe(0);
    expect(recipe.version).toBe(2);
    const plain = withVersion({ ...newRecipe('Plain', { ...clip('A'), blend: { frames: 4 } }) });
    expect(plain.version).toBe(1);
    expect(usesV2(plain)).toBe(false);
  });

  it('leaves a grip link out of the motion version', () => {
    const grip: RecipeLayer = {
      ...layer('g', 'grip', 'rightHand', {}),
      pose: { rightHand: [4, 0, -8] },
      from: { entry: 'longsword', revision: 1 },
    };
    const recipe = withVersion({ ...newRecipe('Grip', clip('A')), layers: [grip] });
    const written = recipe.layers[0] as RecipeLayer;
    expect(written.mode).toBe('replace');
    const relinked = {
      ...recipe,
      layers: [{ ...written, from: { entry: 'longsword', revision: 2 } }],
    };
    expect(motionVersionOf(relinked)).toBe(motionVersionOf(recipe));
  });

  it('refuses what the core refuses past its limits', () => {
    const recipe = onThree();
    expect(v2Problem(recipe)).toBeNull();
    const many = Array.from({ length: V2_LIMITS.keys + 1 }, (_, frame) => ({
      clip: 0,
      frame,
      pose: { chest: [0, 1, 0] as [number, number, number] },
    }));
    expect(v2Problem({ ...recipe, layers: [poseKeys('k', many)] })).toContain(
      `at most ${V2_LIMITS.keys} keys`
    );
    expect(
      v2Problem({ ...recipe, time: [{ kind: 'freeze', clip: 5, at: 1, frames: 2 }] })
    ).toContain('clip 6');
    expect(v2Problem({ ...recipe, joined: [{ ...clip('B'), blend: { frames: 40 } }] })).toContain(
      'at most 30'
    );
  });

  it('moves keys and time edits with their clips and refuses a clip taken away under them', () => {
    const recipe = onThree();
    const [a, b, c] = [recipe.base, ...(recipe.joined ?? [])] as [
      RecipeBase,
      RecipeBase,
      RecipeBase,
    ];
    const moved = followClips(recipe, [c, a, b], [2, 0, 1], false, false);
    expect(moved.layers[0]?.keys?.map(k => k.clip)).toEqual([2, 0]);
    expect(moved.time[0]?.clip).toBe(2);
    expect(() => followClips(recipe, [a, c], null, false, false)).toThrow(/Taking out clip 2/);
    expect(() => followClips(recipe, [a, clip('D'), c], [0, null, 2], false, false)).toThrow(
      /Taking out clip 2/
    );
    const sent = followClips(recipe, [a, c], [0, 2], true, true);
    expect(sent.layers).toBe(recipe.layers);
  });
});

describe('the mock recipe write checks', () => {
  it('refuses a kept index that names another clip than the one sent', () => {
    const r = onThree();
    const [a, b, c] = [r.base, ...(r.joined ?? [])] as RecipeBase[];
    expect(() => followClips(r, [a, c, b] as RecipeBase[], [0, 1, 2], false, false)).toThrow(
      /keeps clip 2/
    );
  });

  it('refuses a recipe too big to open again, as the core does', () => {
    const big = { ...newRecipe('Big', clip('A')), name: 'x'.repeat(300 * 1024) };
    expect(() => checkSize(big)).toThrow(/too big to save/);
    expect(() => checkSize(onThree())).not.toThrow();
  });
});
