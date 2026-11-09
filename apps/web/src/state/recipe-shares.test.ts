import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { moveShare, placesOfMove } from './recipe-shares';

interface Case {
  name: string;
  from: number[];
  to: number[];
  places: (number | null)[];
  change: [number, number];
  want?: [number, number];
  dropped?: number;
  split?: number;
}

/** The core's cases (`recipes/fixtures/clip_moves.json`), which its own tests run too. */
const FIXTURE = resolve(
  import.meta.dirname,
  '../../../../crates/maquette-model/src/recipes/fixtures/clip_moves.json'
);
const cases = (JSON.parse(readFileSync(FIXTURE, 'utf8')) as { cases: Case[] }).cases;

describe('a change following its clips, as the core moves it', () => {
  it.each(cases.map(c => [c.name, c] as const))('%s', (_, c) => {
    const got = moveShare(c.from, c.to, c.places, c.change[0], c.change[1]);
    const want = c.want
      ? { start: c.want[0], end: c.want[1] }
      : c.dropped != null
        ? { dropped: c.dropped }
        : { split: c.split };
    expect(got).toEqual(want);
  });

  it('names where each clip goes when one moves', () => {
    expect(placesOfMove(3, 2, 0)).toEqual([1, 2, 0]);
    expect(placesOfMove(3, 0, 2)).toEqual([2, 0, 1]);
  });
});
