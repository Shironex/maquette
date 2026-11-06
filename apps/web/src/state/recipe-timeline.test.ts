import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { RecipeFile } from '@/ipc/client';
import { frameOfShare, timeline, type ClipSample } from './recipe-timeline';

interface Probe {
  t: number;
  from: Side;
  to: Side | null;
  blend: number;
  share: number;
}
interface Side {
  clip: number;
  at: number;
  hold: { at: number; weight: number; breathe: number } | null;
}
interface Case {
  name: string;
  recipe: RecipeFile;
  fps: number;
  frames: number;
  seams: unknown[];
  clips: unknown[];
  probes: Probe[];
}

/** The core's timeline cases (`recipes/fixtures/timeline.json`), which its own tests run too. */
const FIXTURE = resolve(
  import.meta.dirname,
  '../../../../crates/maquette-model/src/recipes/fixtures/timeline.json'
);
const cases = (JSON.parse(readFileSync(FIXTURE, 'utf8')) as { cases: Case[] }).cases;
const r = (v: number) => Math.round(v * 1e6) / 1e6;
const side = (s: ClipSample): Side => ({
  clip: s.clip,
  at: r(s.at),
  hold: s.hold ? { at: s.hold.at, weight: r(s.hold.weight), breathe: r(s.hold.breathe) } : null,
});

describe('the recipe timeline, as the core makes it', () => {
  it.each(cases.map(c => [c.name, c] as const))('%s plays the frames the bake makes', (_, c) => {
    const t = timeline(c.recipe);
    expect(t.fps).toBe(c.fps);
    expect(t.frames).toBe(c.frames);
    expect(t.seams).toEqual(c.seams);
    expect(t.clips).toEqual(c.clips);
    for (const p of c.probes) {
      const k = t.keys[p.t];
      expect(k, `frame ${p.t}`).toBeDefined();
      if (!k) continue;
      expect({
        t: p.t,
        from: side(k.from),
        to: k.to ? side(k.to) : null,
        blend: r(k.blend),
        share: r(k.share),
      }).toEqual(p);
    }
  });

  it('finds the frame a share stands on', () => {
    const plain = cases[0];
    expect(plain).toBeDefined();
    if (!plain) return;
    const t = timeline(plain.recipe);
    expect(frameOfShare(t, 0.5)).toBe(30);
    expect(frameOfShare(t, 1)).toBe(60);
  });
});
