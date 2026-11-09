import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { KeyEase, RecipeFile } from '@/ipc/client';
import { axisWords, jointLimits, limitLine, overLimits } from './joint-limits';
import { moveKey, removeKey, setEase, setTurn, mirrorPose, emptyKeys } from './recipe-key-edits';
import { clipFrameAt, ease, keyFrame, keyShare, poseAt, shareAt, type Turn } from './recipe-keys';
import { timeline } from './recipe-timeline';

interface Hit {
  layer: string;
  clip: number;
  frame: number;
  bakeFrame: number | null;
  hit: { bone: string; axis: string; word: string; degrees: number; limit: number };
}
interface Case {
  name: string;
  recipe: RecipeFile;
  keys: { clip: number; frame: number; share: number; bakeFrame: number | null }[];
  probes: {
    frame: number;
    share: number;
    clip: number;
    at: number;
    turns: { layer: string; bone: string; turn: Turn }[];
  }[];
  clipFrames: { frame: number; clip: number; at: number }[];
  limits: { judged: number; hits: Hit[] };
}
interface Fixture {
  cases: Case[];
  eases: { ease: KeyEase; at: number[] }[];
  limits: { bone: string; limits: unknown; words: string[] }[];
}

/** The core's pose key cases (`recipes/fixtures/pose_keys.json`), which its own tests and the bake run too. */
const FIXTURE = resolve(
  import.meta.dirname,
  '../../../../crates/maquette-model/src/recipes/fixtures/pose_keys.json'
);
const fixture = JSON.parse(readFileSync(FIXTURE, 'utf8')) as Fixture;
const r = (v: number) => Math.round(v * 1e6) / 1e6;

describe('pose keys against the core', () => {
  it('eases as the core does', () => {
    for (const { ease: kind, at } of fixture.eases)
      expect(
        [0, 0.25, 0.5, 0.75, 1].map(t => r(ease(kind, t))),
        kind
      ).toEqual(at);
  });

  it('has the same joint limits and words', () => {
    for (const row of fixture.limits) {
      expect(jointLimits(row.bone), row.bone).toEqual(row.limits);
      expect(axisWords(row.bone), row.bone).toEqual(row.words);
    }
  });

  for (const c of fixture.cases) {
    describe(c.name, () => {
      const t = timeline(c.recipe);

      it('stands each key where the core does', () => {
        for (const k of c.keys) {
          expect(r(keyShare(c.recipe, k.clip, k.frame))).toBeCloseTo(k.share, 5);
          expect(keyFrame(t, k.clip, k.frame)).toBe(k.bakeFrame);
        }
      });

      it('blends the keys as the core does', () => {
        for (const p of c.probes) {
          expect(shareAt(c.recipe, t, p.frame)).toBeCloseTo(p.share, 5);
          expect(clipFrameAt(t, p.frame)).toEqual({ clip: p.clip, frame: p.at });
          const turns = poseAt(c.recipe, t, p.frame);
          expect(turns.map(x => `${x.layer} ${x.bone}`)).toEqual(
            p.turns.map(x => `${x.layer} ${x.bone}`)
          );
          turns.forEach((x, i) =>
            x.turn.forEach((v, k) =>
              expect(v, `frame ${p.frame} ${x.bone}`).toBeCloseTo(p.turns[i]!.turn[k]!, 4)
            )
          );
        }
      });

      it('reads the clip frame under each bake frame as the core does', () => {
        for (const f of c.clipFrames)
          expect(clipFrameAt(t, f.frame)).toEqual({ clip: f.clip, frame: f.at });
      });

      it('finds the same keyed turns past their limits', () => {
        const hits = c.recipe.layers.flatMap(layer =>
          (layer.keys ?? []).flatMap(key =>
            Object.entries(key.pose).flatMap(([bone, turn]) =>
              overLimits(bone, turn.map(v => v ?? 0) as Turn).map(h => ({
                layer: layer.id,
                bone,
                axis: h.axis,
                degrees: h.degrees,
              }))
            )
          )
        );
        expect(hits.sort((a, b) => a.bone.localeCompare(b.bone))).toEqual(
          c.limits.hits
            .map(h => ({
              layer: h.layer,
              bone: h.hit.bone,
              axis: h.hit.axis,
              degrees: h.hit.degrees,
            }))
            .sort((a, b) => a.bone.localeCompare(b.bone))
        );
      });
    });
  }
});

describe('joint limits line', () => {
  it('says each axis used against the limit on its side', () => {
    expect(limitLine('chest', [12, 18, 0], 'Chest')).toBe(
      'Chest bend 12 of 40 degrees, twist 18 of 45.'
    );
    expect(limitLine('chest', [-12, 0, 0], 'Chest')).toBe('Chest bend 12 of 25 degrees.');
    expect(limitLine('hips', [10, 0, 0], 'Hips')).toBeNull();
    expect(limitLine('chest', [0, 0, 0], 'Chest')).toBeNull();
  });
});

describe('key edits', () => {
  const base = emptyKeys('l-x');
  const at = (clip: number, frame: number) => ({ clip, frame });

  it('keys a bone, sorted, and removes empty keys', () => {
    let l = setTurn(base, at(1, 4), 'head', [0, 10, 0]);
    l = setTurn(l, at(0, 9), 'chest', [5, 0, 0]);
    l = setTurn(l, at(0, 9), 'head', [0, 0, 3]);
    expect(l.keys!.map(k => [k.clip, k.frame])).toEqual([
      [0, 9],
      [1, 4],
    ]);
    expect(Object.keys(l.keys![0]!.pose).sort()).toEqual(['chest', 'head']);
    l = removeKey(l, at(1, 4), 'head');
    expect(l.keys).toHaveLength(1);
    expect(removeKey(l, at(0, 9), null).keys).toEqual([]);
  });

  it('moves, copies and keeps the ease of the key it lands on', () => {
    let l = setTurn(base, at(0, 2), 'head', [0, 10, 0]);
    l = setEase(l, at(0, 2), 'step');
    l = setTurn(l, at(0, 8), 'chest', [1, 0, 0]);
    const copied = moveKey(l, at(0, 2), at(0, 5), { copy: true });
    expect(copied.keys!.map(k => [k.frame, k.ease])).toEqual([
      [2, 'step'],
      [5, 'step'],
      [8, 'smooth'],
    ]);
    const moved = moveKey(l, at(0, 2), at(0, 8));
    expect(moved.keys).toHaveLength(1);
    expect(moved.keys![0]!.ease).toBe('smooth');
    expect(Object.keys(moved.keys![0]!.pose).sort()).toEqual(['chest', 'head']);
  });

  it('mirrors a pose across the body', () => {
    expect(mirrorPose({ leftUpperArm: [10, 20, -30], head: [5, 15, 0] })).toEqual({
      rightUpperArm: [10, -20, 30],
      head: [5, -15, 0],
    });
  });
});
