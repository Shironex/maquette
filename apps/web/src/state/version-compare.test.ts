import { describe, expect, it } from 'vitest';
import type { InspectModel, ModelParts, ProviderJob, RealSize } from '@/ipc/client';
import type { Stop } from './inspector';
import {
  biggestStop,
  jobForVersion,
  pixelsAt,
  realSizeMatch,
  versionRows,
  versionTitle,
  type VersionFacts,
} from './version-compare';

const numbers = (over: Partial<ModelParts> = {}): ModelParts => ({
  parts: [],
  drawCalls: 3,
  triangles: 18_200,
  textures: 1,
  maxTexture: 2048,
  bytes: 1_100_000,
  checks: { pass: 6, warning: 0, fail: 0 },
  ...over,
});
const model = (over: Partial<InspectModel>) =>
  ({
    gpuBytes: 9_400_000,
    bytes: 1_100_000,
    triangles: 18_200,
    size: [0.3, 0.6, 0.3],
    ...over,
  }) as InspectModel;
const job = (over: Partial<ProviderJob>) =>
  ({
    asset: 'lamp',
    provider: 'tripo',
    credits: 30,
    estimate: 30,
    version: 3,
    ...over,
  }) as ProviderJob;
const stop = (z: number): Stop => ({
  name: `Stop ${z}`,
  position: [0, 0, z],
  target: [0, 0, 0],
  fov: 35,
  poster: null,
});
const size: RealSize = { axis: 'height', value: 60, unit: 'cm' };

const tripo: VersionFacts = {
  version: 3,
  numbers: numbers(),
  model: model({}),
  job: job({}),
};
const meshy: VersionFacts = {
  version: 4,
  numbers: numbers({
    drawCalls: 1,
    triangles: 9_600,
    bytes: 600_000,
    maxTexture: 512,
    checks: { pass: 5, warning: 1, fail: 0 },
  }),
  model: model({ gpuBytes: 5_100_000, size: [0.3, 0.3, 0.3] }),
  job: job({ provider: 'meshy', credits: 20, version: 4 }),
};

describe('version compare', () => {
  it('names a version by the send that made it', () => {
    expect(versionTitle(tripo)).toBe('Tripo v3');
    expect(versionTitle({ version: 5, job: null })).toBe('v5');
    expect(jobForVersion([job({})], 'lamp', 3)?.provider).toBe('tripo');
    expect(jobForVersion([job({})], 'lamp', 9)).toBeNull();
  });

  it('puts a dot on the better side of every row, and none where a side is unknown', () => {
    const rows = versionRows(tripo, meshy, { name: 'Desk', pixels: 900 }, size);
    const better = Object.fromEntries(rows.map(r => [r.label, r.better]));
    expect(better).toEqual({
      Triangles: 'right',
      'Draw calls': 'right',
      'GPU memory': 'right',
      Transfer: 'right',
      'Real size match': 'left',
      'Texture detail at Desk': 'left',
      'Inbox checks': 'left',
      Credits: 'right',
    });
    const none = versionRows(tripo, { ...meshy, job: null, numbers: null }, null, null);
    const row = (label: string) => none.find(r => r.label === label)!;
    expect(row('Credits')).toMatchObject({ right: 'no send', better: null });
    expect(row('Real size match')).toMatchObject({ left: 'no real size set', better: null });
    expect(row('Draw calls')).toMatchObject({ right: '-', better: null });
  });

  it('reads the real size along its axis', () => {
    expect(realSizeMatch(model({}), size)).toBeCloseTo(1);
    expect(realSizeMatch(model({ size: [0.3, 0.3, 0.3] }), size)).toBeCloseTo(0.5);
    expect(realSizeMatch(model({ size: [null, null, null] }), size)).toBeNull();
    expect(realSizeMatch(model({}), null)).toBeNull();
  });

  it('finds the stop where the model stands biggest', () => {
    expect(pixelsAt(1, [0, 0, 0], stop(2))).toBeGreaterThan(pixelsAt(1, [0, 0, 0], stop(8)));
    expect(biggestStop([stop(8), stop(2), stop(5)], [0, 0, 0], 1)).toBe(1);
    expect(biggestStop([], [0, 0, 0], 1)).toBeNull();
  });
});
