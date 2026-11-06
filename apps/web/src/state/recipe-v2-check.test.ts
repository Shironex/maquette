import { describe, expect, it } from 'vitest';
import type { RecipeBase, TimeEdit } from '@/ipc/client';
import { baseProblems, blendMost, clipsProblem, timeProblem } from './recipe-v2-check';

const clip = (name: string, frames: number, more: Partial<RecipeBase> = {}): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
  ...more,
});

describe('the version 2 checks of clips and time edits', () => {
  it('lets a blend last at most half the shorter clip as trimmed', () => {
    const clips = [clip('A', 72, { trim: [6, 58] }), clip('B', 64, { trim: [4, 60] })];
    expect(blendMost(clips, 1)).toBe(26);
    const long = [clips[0]!, { ...clips[1]!, blend: { frames: 30 } }];
    expect(clipsProblem(long)).toBe(
      'The blend into clip 2 lasts 30 frames; it can last at most 26, half the shorter clip as trimmed.'
    );
    expect(baseProblems(long, []).clips).toEqual([null, expect.stringContaining('clip 2')]);
  });

  it('marks a trim that keeps nothing on its own clip', () => {
    const p = baseProblems([clip('A', 30), clip('B', 30, { trim: [20, 10] })], []);
    expect(p.clips[1]).toMatch(/^The clip 2 keeps frames 20 to 10/);
    expect(p.clips[0]).toBeNull();
  });

  it('marks retimes out of range and holds that overlap, on the later one', () => {
    const clips = [clip('A', 60)];
    const time: TimeEdit[] = [
      { kind: 'retime', clip: 0, from: 10, to: 20, speed: 5 },
      { kind: 'freeze', clip: 0, at: 30, frames: 4 },
      { kind: 'hold-over', clip: 0, at: 28, frames: 6 },
    ];
    const p = baseProblems(clips, time);
    expect(p.time[0]).toBe('A retime plays 0.25x to 4x, not 5x.');
    expect(p.time[1]).toBeNull();
    expect(p.time[2]).toBe('Two holds on clip 1 overlap. Keep each hold apart.');
    expect(timeProblem(time, clips)).toBe(p.time[0]);
  });
});
