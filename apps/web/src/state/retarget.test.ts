import { describe, expect, it } from 'vitest';
import type { BonePair, ClipReview, ClipSetting } from '@/ipc/client';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import {
  canPin,
  chosenTotals,
  mapCounts,
  playLine,
  reviewCounts,
  reviewOrder,
  slideText,
  stepIndex,
  trimmed,
  withSetting,
} from './retarget';

const items = animationClips().map(clip => ({ clip, file: `/lib/${clip.id}`, uses: [] }));

const setting: ClipSetting = {
  clip: 'standing-dodge-left',
  name: 'dodge_left',
  start: 4,
  end: 33,
  loops: false,
  rootMotion: true,
  speed: 1.15,
};

const review = (clip: string, status: ClipReview['status']): ClipReview => ({
  clip,
  name: clip,
  status,
  note: '',
  frames: 30,
  travel: null,
  slideCm: 0,
  slideFrames: null,
  unpinnedCm: null,
});

describe('retarget rules', () => {
  it('totals the chosen clips by pack, motion and size', () => {
    const totals = chosenTotals(items, [
      'standing-dodge-left',
      'sword-and-shield-run',
      'sword-and-shield-idle',
    ]);
    expect(totals).toMatchObject({ clips: 3, rootMotion: 2, loops: 2, frames: 38 + 22 + 120 });
    expect(totals.packs).toEqual([
      ['Pro Sword and Shield Pack', 2],
      ['Singles', 1],
    ]);
    expect(totals.bytes).toBe((46 + 31 + 118) * 1000);
  });

  it('counts bones that need a look and the ones picked by hand', () => {
    const map: BonePair[] = [
      { bone: 'Hips', source: 'mixamorig:Hips', target: 'hips', how: 'exact' },
      { bone: 'Neck', source: 'mixamorig:Neck', target: null, how: 'missing' },
      { bone: 'Head', source: 'mixamorig:Head', target: 'head', how: 'picked' },
    ];
    expect(mapCounts(map)).toEqual({ matched: 2, look: 1, picked: 1 });
  });

  it('says how a trimmed, faster clip plays', () => {
    expect(playLine(setting, { fps: 30 })).toBe('30 f, plays in 0.87 s at 1.15x');
    expect(trimmed(setting, { frames: 38 })).toBe(true);
    expect(trimmed({ ...setting, start: 0, end: 37 }, { frames: 38 })).toBe(false);
    expect(withSetting([setting], 'standing-dodge-left', { loops: true })[0]!.loops).toBe(true);
  });

  it('counts and orders the review, looks first', () => {
    const clips = [review('a', 'good'), review('b', 'failed'), review('c', 'look')];
    expect(reviewCounts(clips)).toEqual({ good: 1, look: 1, failed: 1 });
    expect(reviewOrder(clips).map(c => c.clip)).toEqual(['c', 'b', 'a']);
    expect(stepIndex('settings')).toBe(2);
  });

  it('offers Pin the feet only for a sliding clip that is not pinned yet', () => {
    const sliding = {
      ...review('run', 'look'),
      slideCm: 4.2,
      slideFrames: [9, 14] as [number, number],
    };
    expect(canPin(sliding)).toBe(true);
    expect(canPin({ ...sliding, unpinnedCm: 4.2, slideCm: 2.1 })).toBe(false);
    expect(canPin(review('idle', 'look'))).toBe(false);
    expect(canPin(review('idle', 'good'))).toBe(false);
    expect(slideText(4.25)).toBe('4.3 cm');
  });
});
