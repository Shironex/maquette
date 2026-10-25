import { describe, expect, it } from 'vitest';
import { loadOrder, MODELS, STOPS, type ModelEntry } from './manifest';
import { STOP_IDS } from './stops';

const entry = (id: string, file: string, stops: ModelEntry['stops']): ModelEntry => ({
  id,
  file,
  position: [0, 0, 0],
  anchor: 'origin',
  stops,
});

describe('loadOrder', () => {
  const models = [
    entry('lamp', 'lamp.glb', ['ship']),
    entry('paper', 'paper.glb', ['concept']),
    entry('crumbs-a', 'crumbs.glb', ['model']),
    entry('crumbs-b', 'crumbs.glb', ['concept']),
    entry('island', 'island.glb', ['refs']),
  ];

  it('loads the current stop first, then the nearest stops, in manifest order', () => {
    expect(loadOrder(models, 'concept')).toEqual([
      'paper.glb',
      'crumbs.glb',
      'island.glb',
      'lamp.glb',
    ]);
  });

  it('loads a shared file once, at its earliest place', () => {
    const order = loadOrder(models, 'model');
    expect(order.filter(file => file === 'crumbs.glb')).toHaveLength(1);
    expect(order[0]).toBe('crumbs.glb');
  });
});

describe('manifest', () => {
  it('has one camera stop per site stop, in order', () => {
    expect(STOPS.map(stop => stop.id)).toEqual([...STOP_IDS]);
  });

  it('has unique model ids and only known stops', () => {
    const ids = MODELS.map(model => model.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const model of MODELS) {
      expect(model.stops.length).toBeGreaterThan(0);
      for (const stop of model.stops) expect(STOP_IDS).toContain(stop);
    }
  });
});
