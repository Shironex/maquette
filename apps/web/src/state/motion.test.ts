import { describe, expect, it } from 'vitest';
import type { Clip, MotionModel, MotionView } from '@/ipc/client';
import {
  EMPTY_DRAFT,
  advance,
  budgetLine,
  budgetTip,
  draftOf,
  heaviestClips,
  modelLine,
  readout,
  shippedMotion,
  ticks,
  withShip,
} from './motion';

const clip = (name: string, bytes: number, patch: Partial<Clip> = {}): Clip => ({
  name,
  index: 0,
  duration: 1.07,
  keys: 32,
  frames: 32,
  fps: 30,
  channels: 3,
  targets: 3,
  bytes,
  shippedBytes: null,
  rootTravel: null,
  loops: true,
  ...patch,
});

const model = (patch: Partial<MotionModel> = {}): MotionModel => ({
  asset: 'figure',
  name: 'Figure',
  kind: 'character',
  version: 1,
  file: '/p/models/figure_v1.glb',
  path: 'models/figure_v1.glb',
  thumbnail: null,
  addedAt: '2026-10-26T10:00:00Z',
  bytes: 1_200_000,
  measured: 'source',
  rig: null,
  clips: [clip('Walk', 400_000), clip('Wave', 100_000)],
  motionBytes: 500_000,
  parts: ['Figure'],
  error: null,
  ...patch,
});

const view = (m: MotionModel, targetMb = 1): MotionView => ({
  project: 'p',
  models: [m],
  states: {},
  clipsOff: { figure: ['Wave', 'Walk'], other: [] },
  stops: [],
  budget: {
    sceneBytes: m.bytes,
    sceneTargetMb: targetMb,
    motionBytes: m.motionBytes,
    firstView: {
      bytes: m.bytes,
      targetMb: 3,
      models: [],
    } as unknown as MotionView['budget']['firstView'],
    firstViewMotion: 0,
  },
  clipsOffSupported: true,
});

describe('motion', () => {
  it('drafts the clips kept off sorted, without empty lists', () => {
    expect(draftOf(view(model())).clipsOff).toEqual({ figure: ['Walk', 'Wave'] });
  });

  it('counts the clips inside the scene budget and drops the ones kept off', () => {
    const m = model();
    const over = budgetLine(view(m), EMPTY_DRAFT);
    expect(over).toMatchObject({ over: true, overBy: 200_000, motion: 500_000 });
    const off = withShip(EMPTY_DRAFT, 'figure', 'Walk', false);
    expect(shippedMotion(m, off)).toBe(100_000);
    const fits = budgetLine(view(m), off);
    expect(fits).toMatchObject({ over: false, overBy: 0, motion: 100_000 });
    expect(budgetTip(fits)).toContain('Clips are 100 KB of it');
    expect(withShip(off, 'figure', 'Walk', true)).toEqual({ ...EMPTY_DRAFT, clipsOff: {} });
  });

  it('lists the heaviest clips that ship first', () => {
    const off = withShip(EMPTY_DRAFT, 'figure', 'Walk', false);
    expect(heaviestClips(model(), EMPTY_DRAFT).map(c => c.name)).toEqual(['Walk', 'Wave']);
    expect(heaviestClips(model(), off).map(c => c.name)).toEqual(['Wave']);
  });

  it('describes a model by its rig, clips, states or parts', () => {
    expect(modelLine(model(), EMPTY_DRAFT)).toBe('no rig · 2 clips');
    expect(modelLine(model({ clips: [], parts: ['A', 'B'] }), EMPTY_DRAFT)).toBe(
      '2 parts · no rig'
    );
  });

  it('reads out the frame and steps the playhead round the loop', () => {
    expect(readout(clip('Walk', 1), 0.6)).toBe('18 / 32 f · 0.60 / 1.07 s');
    expect(advance(0.9, 0.2, [0, 1], true).time).toBeCloseTo(0.1);
    expect(advance(0.9, 0.2, [0, 1], false)).toEqual({ time: 1, playing: false });
    expect(ticks(32)).toEqual([0, 8, 16, 24, 32]);
    expect(ticks(0)).toEqual([0]);
  });
});
