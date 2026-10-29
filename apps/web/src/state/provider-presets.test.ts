import { describe, expect, it } from 'vitest';
import { defaultConfig, providerCatalog } from '@/ipc/mock/fixtures';
import {
  meshyProblem,
  meshySummary,
  presetProblems,
  tripoFaceRange,
  tripoProblem,
  tripoSummary,
} from './provider-presets';

describe('provider presets', () => {
  const catalog = providerCatalog();
  const { tripo, meshy } = defaultConfig().providers;

  it('accepts the default presets', () => {
    expect(presetProblems(defaultConfig(), catalog)).toEqual({});
  });

  it('follows the face limits of each Tripo model', () => {
    const p1 = { ...tripo.prop, model: 'P1-20260311', faceLimit: 30000 };
    expect(tripoProblem(p1, catalog.tripoModels)).toBe(
      'Tripo P1 takes a face limit from 50 to 20,000, or 0 for adaptive.'
    );
    expect(tripoProblem({ ...p1, faceLimit: 0 }, catalog.tripoModels)).toBeNull();
    expect(tripoFaceRange({ ...tripo.prop, smartLowPoly: true })).toEqual([500, 20000]);
    expect(tripoFaceRange({ ...tripo.prop, geometryQuality: 'detailed' })).toEqual([1, 2000000]);
    const many = { ...tripo.character, animations: ['a', 'b', 'c', 'd', 'e', 'f'] };
    expect(tripoProblem(many, catalog.tripoModels)).toMatch(/at most 5/);
  });

  it('keeps Smart Topology to triangles and its own face range', () => {
    const smart = { ...meshy.prop, aiModel: 'meshy-t2.5', targetPolycount: 40000 };
    expect(meshyProblem(smart, catalog.meshyModels)).toBe(
      'Meshy T2.5 aims for 100 to 30,000 faces.'
    );
    const quads = { ...smart, targetPolycount: 15000, topology: 'quad' as const };
    expect(meshyProblem(quads, catalog.meshyModels)).toMatch(/only make triangles/);
  });

  it('summarizes a preset in one line', () => {
    expect(tripoSummary(tripo.character, catalog.tripoModels)).toBe(
      'Tripo H3.1, 50,000 faces, PBR, multi-view, rig with Mixamo bones'
    );
    expect(meshySummary(meshy.prop, catalog.meshyModels)).toBe('Meshy 7.1, PBR, 2K');
    expect(tripoSummary({ ...tripo.character, rigSpec: 'vrm' }, catalog.tripoModels)).toMatch(
      /rig with VRM bones$/
    );
  });

  it('takes VRM bone names only from a 3.0 rig model', () => {
    const vrm = { ...tripo.character, rigSpec: 'vrm' as const };
    expect(tripoProblem(vrm, catalog.tripoModels)).toBeNull();
    const old = { ...vrm, rigModel: 'v2.5-20260210' };
    expect(tripoProblem(old, catalog.tripoModels)).toMatch(/Rig 3.0/);
    expect(tripoProblem({ ...old, rig: false }, catalog.tripoModels)).toBeNull();
  });
});
