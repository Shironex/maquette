import { describe, expect, it } from 'vitest';
import type { AnimationItem, ImportPlan, PlanClip } from '@/ipc/client';
import { animationClips, restBones } from '@/ipc/mock/animation-fixtures';
import {
  NO_ANIMATION_FILTERS,
  animationStats,
  filterClips,
  goesIn,
  importTotals,
  lengthAtLine,
  packCounts,
  restPoseLines,
  rigCounts,
  rootMotionLine,
} from './animations';

const items: AnimationItem[] = animationClips().map(clip => ({
  clip,
  file: `/lib/${clip.id}`,
  uses: [],
}));

const clip = (key: string, status: PlanClip['status'], kb = 40): PlanClip => ({
  key,
  name: key,
  file: `${key}.fbx`,
  pack: 'Singles',
  tags: [],
  rig: 'mixamo',
  bones: 22,
  frames: 30,
  fps: 30,
  length: 1,
  rootMotion: null,
  loops: false,
  bytes: kb * 1000,
  status,
  matched: null,
});

describe('the animation library rules', () => {
  it('filters by pack, tag, rig, root motion, loops and words, newest first', () => {
    const f = NO_ANIMATION_FILTERS;
    expect(filterClips(items, f)).toHaveLength(items.length);
    expect(filterClips(items, { ...f, pack: 'Pro Magic Pack' })).toHaveLength(4);
    expect(filterClips(items, { ...f, tag: 'dodge' }).map(i => i.clip.name)).toContain(
      'Run To Rolling'
    );
    expect(filterClips(items, { ...f, rig: 'tripo' }).map(i => i.clip.name)).toEqual([
      'Duelist lunge',
    ]);
    const loops = filterClips(items, { ...f, loops: true, rootMotion: true });
    expect(loops.every(i => i.clip.loops && i.clip.rootMotion)).toBe(true);
    expect(filterClips(items, { ...f, query: 'shield idle' })).toHaveLength(1);
    const shown = filterClips(items, f);
    expect(shown[0]!.clip.pack).toBe('Singles');
  });

  it('counts packs with Singles after the real packs, and every rig', () => {
    const packs = packCounts(items);
    expect(packs[0]).toEqual(['Pro Sword and Shield Pack', 6]);
    expect(packs.at(-1)).toEqual(['Singles', 4]);
    expect(rigCounts(items)).toEqual([
      ['mixamo', 14],
      ['tripo', 1],
      ['custom', 1],
    ]);
    const stats = animationStats(items, 23_600_000);
    expect(stats).toEqual({ clips: 16, packs: 4, singles: 4, disk: 23_600_000 });
  });

  it('says how long a clip is and where its root goes', () => {
    const dodge = items.find(i => i.clip.name === 'Standing Dodge Left')!.clip;
    expect(lengthAtLine(dodge)).toBe('38 f · 1.27 s at 30 fps');
    expect(rootMotionLine(dodge)).toBe('yes, 0.92 m to the left');
    const run = items.find(i => i.clip.name === 'Run To Rolling')!.clip;
    expect(rootMotionLine(run)).toBe('yes, 2.26 m forward');
    expect(rootMotionLine({ rootMotion: null })).toBe('no, in place');
  });

  it('fits the rest pose into the square, standing upright', () => {
    const lines = restPoseLines(restBones('mixamo'), 100);
    expect(lines).toHaveLength(22);
    const ys = lines.flatMap(l => [l.y1, l.y2]);
    const xs = lines.flatMap(l => [l.x1, l.x2]);
    expect(Math.min(...xs)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...xs)).toBeLessThanOrEqual(100);
    // The head is above the hips: smaller y on screen.
    expect(lines[5]!.y1).toBeLessThan(lines[0]!.y1);
    expect(Math.max(...ys)).toBeLessThanOrEqual(100);
    expect(restPoseLines([], 100)).toEqual([]);
  });

  it('adds new clips and revisions, and duplicates only when kept both', () => {
    const plan: ImportPlan = {
      token: 'import-1',
      files: [
        {
          name: 'Pack.zip',
          archive: true,
          bytes: 1,
          fbx: 3,
          pack: 'Pack',
          rig: 'mixamo',
          bones: 22,
          prefix: 'mixamorig:',
          fps: 30,
          skinned: 3,
          clips: [clip('a', 'new'), clip('b', 'duplicate'), clip('c', 'revision')],
          leftOut: [{ file: 'pose.fbx', reason: 'it holds one pose and no keys' }],
        },
      ],
      readBytes: 1,
      storedBytes: 80_000,
      tags: [],
      add: 2,
      leftOut: 1,
    };
    const none = { keepBoth: [], skip: [] };
    expect(importTotals(plan, none)).toEqual({ add: 2, bytes: 80_000, leftOut: 1 });
    expect(goesIn(plan.files[0]!.clips[1]!, { keepBoth: ['b'], skip: [] })).toBe(true);
    expect(importTotals(plan, { keepBoth: ['b'], skip: ['a'] }).add).toBe(2);
  });
});
