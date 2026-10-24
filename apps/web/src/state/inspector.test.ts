import { describe, expect, it } from 'vitest';
import type { InspectView, TextureRow } from '@/ipc/client';
import { inspectView } from '@/ipc/mock/inspect-fixtures';
import { pipelineView } from '@/ipc/mock/pipeline-fixtures';
import {
  FLIGHT_MS,
  FRAME_TARGET_MS,
  MAX_SAMPLES,
  budgetRows,
  budgetTargetNote,
  easeInOutCubic,
  flight,
  formatMb,
  graphCeiling,
  graphPoints,
  modelForBudget,
  modelSubtitle,
  pickSides,
  presetNote,
  pushSample,
  sideModels,
  statCards,
  stopsHelp,
  stopsOf,
  targetY,
  textureLabel,
  textureLine,
  versusLabel,
} from './inspector';

function view(): InspectView {
  const p = pipelineView('moon-island', '/w/projects/moon-island', [], []);
  return inspectView(
    'moon-island',
    '/w/projects/moon-island',
    p.def.presets,
    p.defaults,
    3,
    60_000
  );
}

const texture = (over: Partial<TextureRow>): TextureRow => ({
  name: 'Image 1',
  uses: ['base color'],
  width: 2048,
  height: 2048,
  bytes: 1_200_000,
  codec: 'KTX2 ETC1S',
  gpuBytes: 0,
  ...over,
});

describe('pickSides', () => {
  it('opens on the newest two sides and honours a pick', () => {
    const v = view();
    const pick = pickSides(v, null, null, null)!;
    expect(versusLabel(pick)).toBe('v6 vs v7');
    expect(sideModels(v, pick.right).map(m => m.model)).toEqual([
      'sakura',
      'flower_shrubs',
      'flower_shrubs',
      'flower_shrubs',
      'character',
    ]);
    const swapped = pickSides(v, 'scene', 'scene/v7', 'scene/v6')!;
    expect(swapped.left?.id).toBe('scene/v7');
    expect(swapped.right.id).toBe('scene/v6');
    // The same side on both ends falls back to another one.
    expect(pickSides(v, 'scene', 'scene/v7', 'scene/v7')!.left?.id).toBe('scene/v6');
  });

  it('shows one side alone and nothing without comparisons', () => {
    const v = view();
    const single = {
      ...v,
      comparisons: [{ ...v.comparisons[0]!, sides: [v.comparisons[0]!.sides[1]!] }],
    };
    const pick = pickSides(single, null, null, null)!;
    expect(pick.left).toBeNull();
    expect(versusLabel(pick)).toBe('v7');
    expect(pickSides({ ...v, comparisons: [] }, null, null, null)).toBeNull();
  });
});

describe('the stats panel', () => {
  it('prints textures, subtitles and stat cards', () => {
    expect(textureLine(texture({}))).toBe('2048² · 1.2 MB');
    expect(textureLine(texture({ width: 1024, height: 512, bytes: 640_000 }))).toBe(
      '1024 × 512 · 640 KB'
    );
    expect(textureLine(texture({ width: null }))).toBe('1.2 MB');
    const color = texture({ name: 'Color' });
    const boxes = texture({ name: 'Boxes' });
    expect(textureLabel(color)).toBe('Base color');
    expect(textureLabel(color, [color, boxes])).toBe('Base color, Color');
    expect(textureLabel(texture({ uses: [], name: 'lightmap' }))).toBe('lightmap');

    const character = view().models.find(m => m.model === 'character')!;
    expect(modelSubtitle(character)).toBe('character.glb · 2 clips');
    const cards = statCards(character, 30_000);
    expect(cards.map(c => c.label)).toEqual(['Triangles', 'Textures', 'GPU memory', 'Transfer']);
    expect(cards[0]).toMatchObject({ value: '39.6k', over: true });
    expect(cards[3]!.value).toBe('1.5 MB');
    expect(presetNote(character)).toMatch(/^Locked: Character stays on medium/);
    const shrubs = view().models.find(m => m.model === 'flower_shrubs')!;
    expect(presetNote(shrubs)).toMatch(/workspace defaults/);
  });

  it('turns budgets into bars and finds the model behind a row', () => {
    const v = view();
    const rows = budgetRows(v);
    expect(rows.map(r => [r.model, r.targetMb, r.over])).toEqual([
      ['sakura', 1, false],
      ['flower_shrubs', 1, false],
      ['character', 1, true],
    ]);
    expect(formatMb(rows[2]!.mb)).toBe('1.5');
    expect(formatMb(0.04)).toBe('0.04');
    const right = pickSides(v, null, null, null)!.right;
    expect(modelForBudget(v, right, 'character')?.id).toBe('shipped/character');
    expect(modelForBudget(v, right, 'nothing')).toBeNull();
  });

  it('says which target a budget row is measured against', () => {
    const v = view();
    const one = (explicit: boolean) => ({ ...v.budgets[0]!, explicit, targetMb: 3 });
    const [single] = budgetRows({ ...v, budgets: [one(false)] });
    expect(single!.target).toBe('scene');
    expect(budgetTargetNote(single!, 3)).toMatch(/3\.0 MB scene target applies/);
    const shared = budgetRows({ ...v, budgets: [one(false), { ...one(false), model: 'b' }] });
    expect(shared.map(r => r.target)).toEqual(['share', 'share']);
    expect(budgetTargetNote(shared[0]!, 3)).toMatch(/even share of the 3\.0 MB/);
    const mixed = budgetRows({ ...v, budgets: [one(true), { ...one(false), model: 'b' }] });
    expect(mixed.map(r => r.target)).toEqual(['model', 'share']);
  });
});

describe('the frame time graph', () => {
  it('keeps 30 seconds of samples', () => {
    let samples: number[] = [];
    for (let i = 0; i < MAX_SAMPLES + 10; i += 1) samples = pushSample(samples, i);
    expect(samples).toHaveLength(MAX_SAMPLES);
    expect(samples[0]).toBe(10);
  });

  it('draws samples right aligned under a ceiling with headroom', () => {
    expect(graphCeiling([5, 6])).toBeCloseTo(FRAME_TARGET_MS * 1.25);
    expect(graphCeiling([40])).toBeCloseTo(44);
    expect(graphPoints([], 560, 44)).toBe('');
    const points = graphPoints([0, FRAME_TARGET_MS * 1.25], 560, 44).split(' ');
    // The slowest sample keeps a tenth of headroom above it.
    expect(points[1]).toBe('560,4');
    expect(points[0]).toBe(`${Math.round((560 - 560 / (MAX_SAMPLES - 1)) * 10) / 10},44`);
    expect(targetY([5], 44)).toBeCloseTo(8.8, 1);
  });
});

describe('camera stops', () => {
  it('eases the camera from where it is to the stop', () => {
    expect(easeInOutCubic(0)).toBe(0);
    expect(easeInOutCubic(0.5)).toBe(0.5);
    expect(easeInOutCubic(2)).toBe(1);
    const stop = stopsOf(view())[0]!;
    const from = {
      position: [0, 0, 0] as [number, number, number],
      target: [0, 0, -1] as [number, number, number],
      fov: 30,
    };
    expect(flight(from, stop, 0)).toEqual(from);
    expect(flight(from, stop, FLIGHT_MS)).toEqual({
      position: stop.position,
      target: stop.target,
      fov: stop.fov,
    });
  });

  it('says where the stops came from', () => {
    const v = view();
    expect(stopsHelp(v)).toBe('From stops.json · click to fly');
    expect(
      stopsHelp({ ...v, stops: { source: 'model-cameras', file: 'island.glb', stops: [] } })
    ).toBe('From the cameras in island.glb · click to fly');
    expect(stopsHelp({ ...v, stops: { source: 'none', file: null, stops: [] } })).toBe(
      'No stops yet'
    );
  });
});
