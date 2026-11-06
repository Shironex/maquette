import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeFile } from '@/ipc/client';
import {
  blendBlocks,
  clipBlocks,
  frameReadout,
  rulerMarks,
  statusOf,
  timeBlocks,
  trimBlocks,
} from './recipe-lanes';
import { timeline } from './recipe-timeline';
import { draftTimeline } from './use-recipe-timeline';

const clip = (name: string, frames: number, more: Partial<RecipeBase> = {}): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: true,
  ...more,
});

const recipe: RecipeFile = {
  version: 2,
  name: 'Slash into thrust',
  revision: 1,
  created: '',
  edited: '',
  base: clip('Slash', 72, { trim: [6, 58] }),
  joined: [
    clip('Thrust', 64, { trim: [4, 60], blend: { frames: 8 } }),
    clip('Idle', 90, { trim: [0, 40], blend: { frames: 6 } }),
  ],
  time: [
    { kind: 'retime', clip: 0, from: 20, to: 34, speed: 1.6 },
    { kind: 'freeze', clip: 1, at: 56, frames: 18 },
  ],
  blanks: [],
  layers: [],
  from: null,
};

describe('the recipe lanes', () => {
  it('lays the clips, blends, trims and time edits on the bake frames', () => {
    const t = timeline(recipe);
    const clips = [recipe.base, ...(recipe.joined ?? [])];
    expect(clipBlocks(t, clips).map(b => b.label)).toEqual([
      'Slash 6 to 58',
      'Thrust 4 to 60',
      'Idle 0 to 40',
    ]);
    expect(blendBlocks(t).map(b => b.last - b.first)).toEqual([8, 6]);
    expect(trimBlocks(t, clips)).toHaveLength(3);
    expect(timeBlocks(t, recipe.time ?? [], ['retime']).map(b => b.label)).toEqual(['Retime 1.6x']);
    expect(timeBlocks(t, recipe.time ?? [], ['freeze', 'hold-over'])[0]?.index).toBe(1);
  });

  it('marks the ruler and reads a frame as the canvas does', () => {
    expect(frameReadout(24, 147, 30)).toBe('24 / 147 f · 0.80 / 4.90 s');
    const marks = rulerMarks(147, 30);
    expect(marks.filter(m => m.major).map(m => m.frame)).toEqual([0, 30, 60, 90, 120, 147]);
  });

  it('reads a new clip as found and a kept one through the file', () => {
    const found = { status: 'found' as const, clip: 'a', warning: null };
    const differs = { status: 'hash-differs' as const, clip: 'b', warning: 'x' };
    expect(statusOf(found, [differs], 1)).toBe('hash-differs');
    expect(statusOf(found, [differs], null)).toBe('found');
  });

  it('turns shares into frames as round 1 did on a plain recipe, through the timeline otherwise', () => {
    const plain = draftTimeline({ ...recipe, base: clip('Walk', 40), joined: [], time: [] });
    expect(plain.plain).toBe(true);
    expect(plain.toFrame(0.5)).toBe(20);
    expect(plain.toShare(10)).toBe(0.25);
    const joined = draftTimeline(recipe);
    expect(joined.plain).toBe(false);
    expect(joined.toShare(joined.toFrame(0.5))).toBeCloseTo(0.5, 1);
  });
});
