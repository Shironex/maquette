import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeFile, RecipeLayer } from '@/ipc/client';
import {
  addClip,
  clipsChanged,
  moveClip,
  outsideTrim,
  patchClip,
  recipeOfDraft,
  rekeep,
  removeClip,
  swapSides,
  swapsOf,
} from './recipe-clips';
import { draftOf, newLayer } from './recipe-draft';

const clip = (name: string, frames = 60): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
});

const file = (more: Partial<RecipeFile> = {}): RecipeFile => ({
  version: 2,
  name: 'Slash into thrust',
  revision: 1,
  created: '',
  edited: '',
  base: clip('Slash', 72),
  joined: [{ ...clip('Thrust', 64), blend: { frames: 8, curve: 'smooth', lineUp: 'hips' } }],
  time: [{ kind: 'freeze', clip: 1, at: 56, frames: 18 }],
  blanks: [],
  layers: [],
  from: null,
  ...more,
});

describe('the clips of a recipe draft', () => {
  it('joins a library clip with a blend no longer than the clips allow', () => {
    const draft = addClip(draftOf(file()), clip('Idle', 10));
    expect(draft.clips[2]?.blend).toEqual({
      frames: 5,
      curve: 'smooth',
      lineUp: 'hips-and-facing',
    });
    expect(draft.kept).toEqual([0, 1, null]);
    expect(clipsChanged(draft, file())).toBe(true);
    expect(clipsChanged(draftOf(file()), file())).toBe(false);
  });

  it('moves the time edits with their clip, and the base never keeps a blend', () => {
    const out = moveClip(draftOf(file()), 1, 0);
    if (!('draft' in out)) throw new Error(out.problem);
    const moved = out.draft;
    expect(moved.clips.map(c => c.name)).toEqual(['Thrust', 'Slash']);
    expect(moved.clips[0]).not.toHaveProperty('blend');
    expect(moved.clips[1]?.blend?.frames).toBe(8);
    expect(moved.kept).toEqual([1, 0]);
    expect(moved.time[0]?.clip).toBe(0);
  });

  it('refuses to take out a clip a time edit sits on, and shifts the rest', () => {
    const draft = draftOf(file());
    expect(removeClip(draft, 1)).toEqual({
      problem: 'Clip 2 (Thrust) cannot go: a time edit sits on it. Take those out first.',
    });
    const three = addClip(draft, clip('Idle'));
    const freezeOnIdle = {
      ...three,
      time: [{ kind: 'freeze' as const, clip: 2, at: 4, frames: 2 }],
    };
    const out = removeClip(freezeOnIdle, 1);
    expect('draft' in out && out.draft.time[0]?.clip).toBe(1);
    expect('draft' in out && out.draft.kept).toEqual([0, null]);
  });

  it('keeps a change on the frames it covered when a clip joins, moves or goes', () => {
    // A reach over 0.5 to 0.8 of one 60 frame clip covers its frames 30 to 48.
    const reach = {
      ...newLayer('reach', [], []),
      bone: 'rightHand',
      settings: { ...newLayer('reach', [], []).settings, start: 0.5, end: 0.8 },
    } as RecipeLayer;
    const one = { ...draftOf(file({ base: clip('Idle'), joined: [], time: [] })), layers: [reach] };
    const joined = addClip(one, clip('Wave'));
    // Still clip 1 frames 30 to 48 of 120, not clip 2 frames 4 to 36.
    expect(joined.layers[0]?.settings).toMatchObject({ start: 0.25, end: 0.4 });
    const front = moveClip(joined, 1, 0);
    expect('draft' in front && front.draft.layers[0]?.settings).toMatchObject({
      start: 0.75,
      end: 0.9,
    });
    expect(removeClip(joined, 0)).toEqual({
      problem:
        'Clip 1 (Idle) cannot go: the change Reach on Right hand sits on it. Take those out first.',
    });
    const back = removeClip(joined, 1);
    expect('draft' in back && back.draft.layers[0]?.settings).toMatchObject({
      start: 0.5,
      end: 0.8,
    });
  });

  it('refuses to move a clip away from the clip a change runs across from', () => {
    const across = {
      ...newLayer('wave', [], []),
      bone: 'rightUpperArm',
      settings: { ...newLayer('wave', [], []).settings, start: 0.4, end: 0.6 },
    } as RecipeLayer;
    const two = {
      ...draftOf(file({ base: clip('A'), joined: [], time: [] })),
      layers: [across],
    };
    const joined = addClip(two, clip('B'));
    expect(joined.layers[0]?.settings).toMatchObject({ start: 0.2, end: 0.3 });
    const wide = {
      ...joined,
      layers: [{ ...across, settings: { ...across.settings, start: 0.4, end: 0.6 } }],
    };
    const out = moveClip(wide, 1, 0);
    expect('problem' in out && out.problem).toMatch(/Clip 2 \(B\) cannot move away from clip 1/);
  });

  it('drops a trim of the whole clip, and counts edits the trim leaves out', () => {
    const draft = patchClip(draftOf(file()), 0, { trim: [0, 72] });
    expect(draft.clips[0]).not.toHaveProperty('trim');
    expect(patchClip(draft, 0, { trim: [6, 58] }).clips[0]?.trim).toEqual([6, 58]);
    expect(outsideTrim(file().time ?? [], 1, [4, 50])).toHaveLength(1);
    expect(outsideTrim(file().time ?? [], 1, [4, 60])).toHaveLength(0);
  });

  it('finds again which clip each keeps once the file moved under it', () => {
    const was = [clip('A'), clip('B')];
    const now = [clip('B'), clip('A'), clip('C')];
    expect(rekeep([clip('A'), clip('B'), clip('D')], [0, 1, null], was, now)).toEqual([1, 0, null]);
  });

  it('makes the recipe the draft says, and swaps sides for the mirror', () => {
    const reach = { ...newLayer('reach', [], []), bone: 'rightHand' } as RecipeLayer;
    const look = { ...newLayer('look-at', [], []), bone: 'head' } as RecipeLayer;
    const draft = { ...draftOf(file()), mirror: true, layers: [reach, look] };
    const made = recipeOfDraft(file(), draft);
    expect(made.mirror).toBe(true);
    expect(made.joined?.[0]?.name).toBe('Thrust');
    expect(swapsOf(draft.layers).map(s => s.to)).toEqual(['leftHand']);
    expect(swapSides(draft.layers).map(l => l.bone)).toEqual(['leftHand', 'head']);
  });
});

describe('swapping sides with a grip', () => {
  it('mirrors the grip pose with its hand', () => {
    const grip: RecipeLayer = {
      ...newLayer('reach', [], []),
      kind: 'grip',
      bone: 'rightHand',
      pose: { rightHand: [4, 10, -8], rightIndexProximal: [0, 0, 60] },
    };
    const [swapped] = swapSides([grip]);
    expect(swapped?.bone).toBe('leftHand');
    expect(swapped?.pose).toEqual({ leftHand: [4, -10, 8], leftIndexProximal: [0, 0, -60] });
  });
});
