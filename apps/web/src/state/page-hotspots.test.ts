import { describe, expect, it } from 'vitest';
import type { SceneLayout } from '@/ipc/client';
import { cameraAt, hotspotCount, hotspotsAt, project } from './page-hotspots';

const layout: SceneLayout = {
  version: 1,
  models: {
    cabinet: { position: [0, 0, 0], rotation: [0, 0, 0], scale: 1 },
  },
  stops: [
    { id: 'a', name: 'A', position: [0, 0, 5], target: [0, 0, 0], fov: 40, models: null },
    { id: 'b', name: 'B', position: [0, 0, -5], target: [0, 0, -10], fov: 60, models: null },
  ],
  desk: null,
  states: {
    cabinet: {
      states: [
        { id: 'rest', name: 'Rest', parts: {} },
        { id: 'on', name: 'Breakers on', parts: {} },
      ],
      transitions: [],
      triggers: [
        { kind: 'click', to: 'on', stop: null, part: 'Breaker', start: null, end: null },
        { kind: 'hover', to: 'rest', stop: null, part: null, start: null, end: null },
        { kind: 'stop', to: 'on', stop: 'a', part: null, start: null, end: null },
      ],
    },
  },
  clips: {},
  timeline: { explode: {}, paths: {}, style: null, text: null },
  paths: [],
};

describe('hotspots on the page', () => {
  it('pins the click and hover triggers where the camera sees the prop', () => {
    const spots = hotspotsAt(layout, 0, 1.6);
    expect(spots.map(s => s.label)).toEqual(['Click · Breakers on', 'Hover · Rest']);
    expect(spots[0]!.x).toBeCloseTo(0.5);
    expect(spots[0]!.y).toBeCloseTo(0.5);
    expect(spots[1]!.y).toBeLessThan(spots[0]!.y);
  });

  it('drops a prop behind the camera, but still counts it', () => {
    expect(hotspotsAt(layout, 1, 1.6)).toEqual([]);
    expect(hotspotCount(layout)).toBe(2);
  });

  it('moves the camera between the stops', () => {
    expect(cameraAt(layout.stops, 0.5)).toMatchObject({ position: [0, 0, 0], fov: 50 });
    expect(cameraAt([], 0)).toBeNull();
  });

  it('places a point off centre, and drops one outside the window', () => {
    const camera = { position: [0, 0, 5] as const, target: [0, 0, 0] as const, fov: 40 };
    const c = {
      ...camera,
      position: [...camera.position],
      target: [...camera.target],
    } as Parameters<typeof project>[0];
    expect(project(c, [1, 0, 0], 1.6)!.x).toBeGreaterThan(0.5);
    expect(project(c, [0, 1, 0], 1.6)!.y).toBeLessThan(0.5);
    expect(project(c, [50, 0, 0], 1.6)).toBeNull();
  });
});
