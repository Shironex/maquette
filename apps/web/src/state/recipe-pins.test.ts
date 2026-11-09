import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { RecipeFile, RecipeLayer } from '@/ipc/client';
import {
  heldFrames,
  pinAmount,
  pinCheckWords,
  pinProblem,
  pinRows,
  pinSettings,
  pinSpans,
  withPinBone,
  type PinSpan,
} from './recipe-pins';
import { timeline } from './recipe-timeline';

interface Case {
  name: string;
  recipe: RecipeFile;
  spans: PinSpan[];
  held: ([number, number] | null)[];
  amounts: { layer: string; frame: number; amount: number }[];
}

/** The core's pin cases (`recipes/fixtures/pins.json`), which its own tests run too. */
const FIXTURE = resolve(
  import.meta.dirname,
  '../../../../crates/maquette-model/src/recipes/fixtures/pins.json'
);
const cases = (JSON.parse(readFileSync(FIXTURE, 'utf8')) as { cases: Case[] }).cases;
const r = (v: number) => Math.round(v * 1e6) / 1e6;

const pin = (over: Partial<RecipeLayer> = {}): RecipeLayer => ({
  id: 'l-1',
  kind: 'pin',
  bone: 'leftFoot',
  settings: {
    weight: 1,
    start: 0.2,
    end: 0.6,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
  },
  ...over,
});

describe('pins, as the core holds them', () => {
  it.each(cases.map(c => [c.name, c] as const))('%s holds on the same frames', (_, c) => {
    const spans = pinSpans(timeline(c.recipe), c.recipe.layers);
    expect(spans).toEqual(c.spans);
    expect(spans.map(heldFrames)).toEqual(c.held);
    for (const a of c.amounts) {
      const span = spans.find(s => s.layer === a.layer)!;
      expect(r(pinAmount(span, a.frame)), `${a.layer} at ${a.frame}`).toBe(a.amount);
    }
  });
});

describe('pin settings in the editor', () => {
  it('starts a foot with toe and heel flat and 3 frame eases, and a hand with no flat', () => {
    const foot = pinSettings(pin().settings, 'leftFoot');
    expect(foot).toMatchObject({ easeIn: 3, easeOut: 3, flat: 'toe-and-heel' });
    expect(pinSettings(pin().settings, 'rightHand').flat).toBeNull();
    const moved = withPinBone(pin({ settings: foot }), 'leftHand');
    expect(moved.settings.flat).toBeNull();
    expect(withPinBone(moved, 'rightFoot').settings.flat).toBe('toe-and-heel');
  });

  it('refuses what the core refuses', () => {
    expect(pinProblem(pin(), [])).toBeNull();
    const aimed = pin({ settings: { ...pin().settings, target: 'wall' } });
    expect(pinProblem(aimed, [])).toMatch(/wall/);
    const flatHand = pin({ bone: 'leftHand', settings: { ...pin().settings, flat: 'heel' } });
    expect(pinProblem(flatHand, [])).toMatch(/Only a foot/);
    const half = pin({ settings: { ...pin().settings, easeIn: 1.5 } });
    expect(pinProblem(half, [])).toMatch(/whole frames/);
  });

  it('stacks pins that overlap on rows of the Pin lane', () => {
    const at = (share: number) => Math.round(share * 100);
    const a = pin({ id: 'a' });
    const b = pin({ id: 'b', bone: 'rightFoot' });
    const c = pin({ id: 'c', settings: { ...pin().settings, start: 0.7, end: 0.9 } });
    const reach: RecipeLayer = { ...pin({ id: 'r' }), kind: 'reach' };
    const rows = pinRows([a, reach, b, c], at);
    expect(rows.map(row => row.map(l => l.id))).toEqual([['a', 'c'], ['b']]);
  });

  it('says whether the limb can stay on its spot', () => {
    const ok = pinCheckWords({ layer: 'l-1', hips: 0.31, limb: 0.88, outFrame: null }, 'leftFoot');
    expect(ok.title).toBe('The foot can stay on its spot');
    expect(ok.detail).toBe(
      'The hips move 31 cm while it holds. The leg reaches 88 cm, so the knee bends and the foot holds.'
    );
    const out = pinCheckWords({ layer: 'l-1', hips: 0.6, limb: 0.6, outFrame: 18 }, 'rightHand');
    expect(out.title).toMatch(/hand cannot/);
    expect(out.detail).toMatch(/At frame 18/);
  });
});
