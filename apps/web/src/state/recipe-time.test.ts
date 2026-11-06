import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeFile } from '@/ipc/client';
import {
  asKind,
  changeWords,
  editSpan,
  frameOfClip,
  lengthChange,
  newHold,
  newRetime,
  playsIn,
  timeTitle,
} from './recipe-time';
import { timeline } from './recipe-timeline';

const clip = (name: string, frames: number, more: Partial<RecipeBase> = {}): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: true,
  ...more,
});

/** The design's example: 147 frames. */
const example: RecipeFile = {
  version: 2,
  name: 'Slash into thrust',
  revision: 1,
  created: '',
  edited: '',
  base: clip('Great Sword Slash', 72, { trim: [6, 58] }),
  joined: [
    clip('Great Sword Thrust', 64, { trim: [4, 60], blend: { frames: 8 } }),
    clip('Sword Idle', 90, { trim: [0, 40], blend: { frames: 6 } }),
  ],
  time: [
    { kind: 'retime', clip: 0, from: 20, to: 34, speed: 1.6 },
    { kind: 'freeze', clip: 1, at: 56, frames: 18 },
  ],
  blanks: [],
  layers: [],
  from: null,
};

describe('time edits', () => {
  it('reads the design example the way the canvas does', () => {
    const t = timeline(example);
    expect(t.frames).toBe(147);
    const [retime, freeze] = example.time ?? [];
    expect(playsIn(t, retime!)).toBe('14 frames play in 9 f');
    expect(changeWords(lengthChange(example, 0))).toBe('5 f shorter');
    expect(changeWords(lengthChange(example, 1))).toBe('18 f longer');
    const span = editSpan(t, freeze!);
    expect(span && span[1] - span[0]).toBe(18);
    expect(timeTitle(retime!)).toBe('Retime 1.6x');
    expect(timeTitle(freeze!)).toBe('Freeze 18 f');
  });

  it('finds the bake frame of a clip frame, or none where the trim cuts it', () => {
    const t = timeline(example);
    expect(frameOfClip(t, 0, 6)).toBe(0);
    expect(frameOfClip(t, 0, 2)).toBeNull();
  });

  it('starts new edits inside the kept frames, and keeps what a kind switch can', () => {
    const slash = example.base;
    const r = newRetime(slash, 0);
    expect(r.from! >= 6 && r.to! <= 58).toBe(true);
    const hold = newHold('hold-over', clip('Short', 10), 0);
    expect(hold.at! + hold.frames!).toBeLessThanOrEqual(10);
    const freeze = asKind(hold, 'freeze', clip('Short', 10));
    expect(freeze).toMatchObject({ kind: 'freeze', at: hold.at, frames: hold.frames });
    expect(asKind(freeze, 'retime', slash).kind).toBe('retime');
  });
});
