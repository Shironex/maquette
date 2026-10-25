import { describe, expect, it } from 'vitest';
import { fromLayout, type SceneLayout, type SiteExtras } from './layout';

const lamp: SiteExtras = { file: 'desk_lamp.glb', anchor: 'origin' };
const island: SiteExtras = { file: 'island.glb', anchor: 'origin', spinAt: 'web' };

const layout = (overrides: Partial<SceneLayout> = {}): SceneLayout => ({
  version: 1,
  models: {
    'desk-lamp': { position: [0.42, 0, -0.1], rotation: [0, -35, 0], scale: 0.4 },
    island: { position: [0, 0.1, 0], rotation: [0, 0, 0], scale: 1 },
    'not-shipped': { position: [0, 0, 0], rotation: [0, 0, 0], scale: 1 },
  },
  stops: [
    { id: 'a', name: 'Overview', position: [0, 0.5, 1], target: [0, 0, 0], fov: 40, models: null },
    {
      id: 'b',
      name: 'Concept',
      position: [0.9, 0.5, 1.4],
      target: [0, 0.15, 0],
      fov: 35,
      models: ['island'],
    },
  ],
  desk: { size: [1.6, 0.8] },
  ...overrides,
});

describe('fromLayout', () => {
  it('maps stops in order onto the site stop ids and lifts them', () => {
    const { stops } = fromLayout(layout(), { assets: {}, lift: 0.9 });
    expect(stops.map(stop => stop.id)).toEqual(['hero', 'concept']);
    expect(stops[1]).toEqual({
      id: 'concept',
      position: [0.9, 1.4, 1.4],
      target: [0, 1.05, 0],
      fov: 35,
    });
  });

  it('keeps placements, adds extras and skips assets the site does not ship', () => {
    const { models } = fromLayout(layout(), { assets: { 'desk-lamp': lamp, island }, lift: 0.9 });
    expect(models.map(model => model.id)).toEqual(['desk-lamp', 'island']);
    expect(models[0]).toMatchObject({
      file: 'desk_lamp.glb',
      position: [0.42, 0.9, -0.1],
      rotation: [0, -35, 0],
      scale: 0.4,
    });
    expect(models[1]?.spinAt).toBe('web');
  });

  it('derives which stops see a model from each stop list, null meaning all', () => {
    const { models } = fromLayout(layout(), { assets: { 'desk-lamp': lamp, island } });
    expect(models.find(model => model.id === 'desk-lamp')?.stops).toEqual(['hero']);
    expect(models.find(model => model.id === 'island')?.stops).toEqual(['hero', 'concept']);
  });

  it('falls back to the hero stop for a model no stop lists', () => {
    const stops = layout().stops.map(stop => ({ ...stop, models: [] }));
    const { models } = fromLayout(layout({ stops }), { assets: { island } });
    expect(models[0]?.stops).toEqual(['hero']);
  });

  it('ignores stops past the site tour', () => {
    const extra = Array.from({ length: 10 }, (_, i) => ({
      ...layout().stops[0]!,
      id: `s${i}`,
    }));
    expect(fromLayout(layout({ stops: extra }), { assets: {} }).stops).toHaveLength(7);
  });

  it('refuses a layout version it does not know', () => {
    expect(() => fromLayout(layout({ version: 2 }), { assets: {} })).toThrow(/version 2/);
  });
});
