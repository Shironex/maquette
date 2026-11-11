import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeLayer, TimeEdit } from '@/ipc/client';
import type { RecipeDraft } from './recipe-draft';
import { applyFix } from './apply-fix';

const clip = (name: string, frames: number): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
});

function draft(): RecipeDraft {
  return {
    layers: [
      {
        id: 'keys',
        kind: 'pose',
        keys: [{ clip: 1, frame: 12, pose: { spine: [10, 50, 0] } }],
      } as unknown as RecipeLayer,
    ],
    blanks: [],
    clips: [clip('Slash', 40), { ...clip('Thrust', 40), blend: { frames: 4 } }],
    kept: [0, 1],
    time: [{ kind: 'hold-over', clip: 0, at: 20, frames: 10, easeIn: 0, easeOut: 0 } as TimeEdit],
    mirror: false,
  };
}

describe('a review fix laid on the editor draft', () => {
  it('widens the blend into the clip it names and leaves the rest', () => {
    const next = applyFix(draft(), { kind: 'blend', clip: 1, frames: 12 });
    expect(next?.clips[1]?.blend?.frames).toBe(12);
    expect(next?.clips[0]).toEqual(draft().clips[0]);
  });

  it('eases a hold over in or out, keeping the other side', () => {
    const next = applyFix(draft(), {
      kind: 'hold-ease',
      clip: 0,
      at: 20,
      easeIn: null,
      easeOut: 6,
    });
    expect(next?.time[0]).toMatchObject({ easeIn: 0, easeOut: 6 });
  });

  it('holds a keyed turn to its limit on the one axis', () => {
    const next = applyFix(draft(), {
      kind: 'key-turn',
      layer: 'keys',
      clip: 1,
      frame: 12,
      bone: 'spine',
      axis: 'y',
      degrees: 45,
    });
    const keys = (next?.layers[0] as { keys: { pose: Record<string, number[]> }[] }).keys;
    expect(keys[0]?.pose.spine).toEqual([10, 45, 0]);
  });

  it('lays nothing when what it names is gone', () => {
    expect(applyFix(draft(), { kind: 'blend', clip: 3, frames: 12 })).toBeNull();
    expect(applyFix(draft(), { kind: 'blend', clip: 0, frames: 12 })).toBeNull();
    expect(
      applyFix(draft(), { kind: 'hold-ease', clip: 0, at: 30, easeIn: 4, easeOut: null })
    ).toBeNull();
    expect(
      applyFix(draft(), {
        kind: 'key-turn',
        layer: 'keys',
        clip: 1,
        frame: 13,
        bone: 'spine',
        axis: 'y',
        degrees: 45,
      })
    ).toBeNull();
  });
});
