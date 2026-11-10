import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type {
  GripFile,
  GripHands,
  GripHolds,
  GripSide,
  RecipeFile,
  RecipeLayer,
} from '@/ipc/client';
import {
  GRIP_EASE_FRAMES,
  WEAPONS,
  WEAPON_GROUPS,
  defaultHands,
  gripAmount,
  gripBones,
  gripOf,
  gripSpans,
  handsOfPose,
  handsWords,
  isGripBone,
  keyedPose,
  layerOf,
  mirrorGrip,
  poseFor,
  updated,
  weaponGroup,
  weaponLabel,
  type GripPose,
  type GripSpan,
} from './recipe-grips';
import { timeline } from './recipe-timeline';

interface Fixture {
  weapons: [string, string, string][];
  groups: [string, string][];
  easeFrames: number;
  grips: {
    grip: GripFile;
    words: string;
    group: string;
    mirrored: GripPose;
    applies: { hands: GripHands | null; bone: string; pose: GripPose }[];
  }[];
  spans: {
    name: string;
    recipe: RecipeFile;
    spans: GripSpan[];
    amounts: { layer: string; frame: number; amount: number }[];
  }[];
  keyed: {
    recipe: RecipeFile;
    layer: string;
    frame: number;
    keep: string[] | null;
    pose: GripPose;
  }[];
  made: {
    pose: GripPose;
    hands: GripHands | null;
    holds: GripSide | null;
    made: { hands: GripHolds; pose: GripPose } | null;
  }[];
}

/** The core's grip cases (`recipes/fixtures/grips.json`), which its own tests run too. */
const FIXTURE = resolve(
  import.meta.dirname,
  '../../../../crates/maquette-model/src/recipes/fixtures/grips.json'
);
const fixture = JSON.parse(readFileSync(FIXTURE, 'utf8')) as Fixture;
const r = (v: number) => Math.round(v * 1e6) / 1e6;
const [longsword, dagger] = fixture.grips.map(c => c.grip) as [GripFile, GripFile];
const link = { entry: 'longsword-two-hands', uid: '0a1b2c3d4e5f6071', revision: 2 };

describe('grips, as the core lays them', () => {
  it('knows the same weapons, groups and default ease', () => {
    expect(WEAPONS).toEqual(fixture.weapons);
    expect(WEAPON_GROUPS).toEqual(fixture.groups);
    expect(GRIP_EASE_FRAMES).toBe(fixture.easeFrames);
  });

  it.each(fixture.grips.map(c => [c.grip.name, c] as const))(
    '%s gives the same hands, mirror and words',
    (_, c) => {
      expect(handsWords(c.grip)).toBe(c.words);
      expect(weaponGroup(c.grip.weapon)).toBe(c.group);
      expect(mirrorGrip(c.grip.pose)).toEqual(c.mirrored);
      for (const apply of c.applies) {
        const taken = poseFor(c.grip, apply.hands);
        expect(`${taken.holds}Hand`, `hands ${apply.hands}`).toBe(apply.bone);
        expect(taken.pose, `hands ${apply.hands}`).toEqual(apply.pose);
      }
    }
  );

  it.each(fixture.spans.map(c => [c.name, c] as const))('%s holds on the same frames', (_, c) => {
    const spans = gripSpans(timeline(c.recipe), c.recipe.layers);
    expect(spans).toEqual(c.spans);
    for (const a of c.amounts) {
      const span = spans.find(s => s.layer === a.layer)!;
      expect(r(gripAmount(span, a.frame)), `${a.layer} at ${a.frame}`).toBe(a.amount);
    }
  });

  it('reads the same hand pose from pose keys at a frame', () => {
    expect(fixture.keyed.length).toBeGreaterThan(2);
    for (const c of fixture.keyed) {
      const layer = c.recipe.layers.find(l => l.id === c.layer)!;
      const pose = keyedPose(c.recipe, timeline(c.recipe), layer, c.frame, c.keep);
      expect(pose, `frame ${c.frame}`).toEqual(c.pose);
    }
  });

  it('makes the same grip from a pose, or refuses the same hands', () => {
    expect(fixture.made.some(c => c.made === null)).toBe(true);
    for (const c of fixture.made)
      expect(gripOf(c.pose, c.hands, c.holds), `${c.hands} ${c.holds}`).toEqual(c.made);
  });
});

describe('grips in the editor', () => {
  it('poses hands, fingers and the arm above them only', () => {
    expect(gripBones('right')).toHaveLength(18);
    expect(gripBones('left').every(isGripBone)).toBe(true);
    for (const bone of ['chest', 'head', 'leftShoulder', 'rightFoot', 'rightIndex', 'hand'])
      expect(isGripBone(bone), bone).toBe(false);
    expect(weaponLabel('longsword')).toBe('Longsword');
    expect(weaponLabel('lightsaber')).toBe('lightsaber');
    expect(weaponGroup('lightsaber')).toBe('other');
  });

  it('lays a copy on the hand that holds, in place of the clip, with a link back', () => {
    const layer = layerOf('l-3', longsword, link);
    expect(layer).toMatchObject({ id: 'l-3', kind: 'grip', bone: 'rightHand', mode: 'replace' });
    expect(layer.pose).toEqual(longsword.pose);
    expect(layer.from).toEqual(link);
    expect(layer.settings).toMatchObject({ start: 0, end: 1, weight: 1, easeIn: 4, easeOut: 4 });
    expect(defaultHands(longsword)).toBe('both');
    expect(defaultHands(dagger)).toBe('right');
    const left = layerOf('l-4', dagger, link, 'left');
    expect(left.bone).toBe('leftHand');
    expect(left.pose).toEqual(mirrorGrip(dagger.pose));
  });

  it('takes a newer revision into a change and keeps where it sat and the hands it posed', () => {
    const was: RecipeLayer = {
      ...layerOf('l-3', longsword, link, 'right'),
      settings: { ...layerOf('l-3', longsword, link).settings, start: 0.2, end: 0.7, weight: 0.8 },
    };
    const newer: GripFile = {
      ...longsword,
      revision: 3,
      pose: { ...longsword.pose, rightIndexProximal: [0, 0, 75] },
    };
    const next = updated(was, newer, { ...link, revision: 3 });
    expect(next.from?.revision).toBe(3);
    expect(next.settings).toEqual(was.settings);
    expect(handsOfPose(next.pose ?? {})).toBe('right');
    expect(next.pose?.rightIndexProximal).toEqual([0, 0, 75]);
    const other = updated(was, newer, link, 'left');
    expect(other.bone).toBe('leftHand');
    expect(handsOfPose({ chest: [0, 0, 0] })).toBeNull();
  });
});
