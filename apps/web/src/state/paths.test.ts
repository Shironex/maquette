import { describe, expect, it } from 'vitest';
import type { LayoutStop, ScenePath } from '@/ipc/client';
import {
  EMPTY_PATHS,
  addEnergize,
  addPath,
  addPoint,
  along,
  energizeLine,
  follow,
  movePoint,
  nextPathName,
  pathBytes,
  pathId,
  pathsProblem,
  removeEnergize,
  removePath,
  removePoint,
  setAnchor,
  setPath,
  snapPoint,
  tubeTris,
  type PathsDraft,
} from './paths';

const stop = (id: string, name: string): LayoutStop => ({
  id,
  name,
  position: [0, 1, 2],
  target: [0, 0, 0],
  fov: 35,
  models: null,
});

const STOPS = [
  stop('arrival', 'Arrival'),
  stop('veil-gate', 'Veil gate'),
  stop('keystone', 'Keystone'),
];

const cable = (): ScenePath => ({
  id: 'cable',
  name: 'Cable',
  curve: 'polyline',
  points: [
    [0, 0, 0],
    [3, 0, 0],
    [3, 4, 0],
  ],
  start: 'shop',
  end: 'crystal-lantern',
  width: 0.04,
  color: '#f2c230',
  glow: '#ffe08a',
});

const wired = (): PathsDraft => ({
  paths: [cable()],
  tracks: { cable: { easing: 'linear', keys: [{ stop: 'arrival', amount: 1 }] } },
});

describe('the paths rules', () => {
  it('names and ids new paths after the ones there', () => {
    const { draft, id } = addPath(EMPTY_PATHS, [0, 0, 0], [1, 0, 0]);
    expect(id).toBe('path-1');
    expect(draft.paths[0]).toMatchObject({ name: 'Path 1', curve: 'catmull-rom', start: null });
    expect(nextPathName(draft.paths)).toBe('Path 2');
    expect(pathId([cable()], 'Cable')).toBe('cable-2');
  });

  it('adds, moves and removes points, keeping two and letting a removed end go', () => {
    const added = addPoint(wired(), 'cable', [3, 4, 2], 1);
    expect(added.index).toBe(2);
    expect(added.draft.paths[0]?.points[2]).toEqual([3, 4, 2]);
    expect(added.draft.paths[0]?.end).toBe('crystal-lantern');

    const past = addPoint(wired(), 'cable', [5, 4, 0]);
    expect(past.index).toBe(3);
    expect(past.draft.paths[0]?.end).toBeNull();

    const moved = movePoint(wired(), 'cable', 1, [2, 0, 1]);
    expect(moved.paths[0]?.points[1]).toEqual([2, 0, 1]);

    const removed = removePoint(wired(), 'cable', 0);
    expect(removed.paths[0]?.points).toHaveLength(2);
    expect(removed.paths[0]?.start).toBeNull();
    const kept = removePoint(removed, 'cable', 0);
    expect(kept.paths[0]?.points).toHaveLength(2);
  });

  it('anchors an end to a model at its spot and follows the model when it moves', () => {
    const anchored = setAnchor(wired(), 'cable', 'end', 'lamp', [1, 2, 3]);
    expect(anchored.paths[0]).toMatchObject({ end: 'lamp' });
    expect(anchored.paths[0]?.points[2]).toEqual([1, 2, 3]);
    const loose = setAnchor(wired(), 'cable', 'start', null, null);
    expect(loose.paths[0]?.start).toBeNull();
    expect(loose.paths[0]?.points[0]).toEqual([0, 0, 0]);

    const place = (x: number) => ({
      position: [x, 0, 0] as [number, number, number],
      rotation: [0, 0, 0] as [number, number, number],
      scale: 1,
    });
    const [after] = follow(
      [cable()],
      { 'crystal-lantern': place(1) },
      { 'crystal-lantern': place(2), shop: place(5) }
    );
    expect(after?.points[2]).toEqual([4, 4, 0]);
    expect(after?.points[0]).toEqual([0, 0, 0]);
  });

  it('keeps the width in range and drops the track of a removed path', () => {
    expect(setPath(wired(), 'cable', { width: 5 }).paths[0]?.width).toBe(1);
    expect(setPath(wired(), 'cable', { name: 'Feed' }).paths[0]?.name).toBe('Feed');
    expect(removePath(wired(), 'cable')).toEqual(EMPTY_PATHS);
  });

  it('snaps points to 5 cm or rounds them to a tenth of a millimetre', () => {
    expect(snapPoint([0.123, 1.071, -0.026], true)).toEqual([0.1, 1.05, -0.05]);
    expect(snapPoint([0.123456, 0, 0], false)).toEqual([0.1235, 0, 0]);
  });

  it('measures the path, its tube and what it adds to layout.json', () => {
    expect(along(cable())).toEqual([0, 3, 7]);
    expect(tubeTris(cable())).toBe(88 * 8 * 2);
    expect(pathBytes(cable(), wired().tracks.cable)).toMatch(/B$/);
  });

  it('energizes from unlit at the first stop to lit at the last, and says so', () => {
    const lit = addEnergize(EMPTY_PATHS, 'cable', STOPS);
    expect(lit.tracks.cable?.keys).toEqual([
      { stop: 'arrival', amount: 0 },
      { stop: 'keystone', amount: 1 },
    ]);
    expect(energizeLine(lit.tracks.cable, STOPS)).toBe('0% at Arrival, 100% at Keystone');
    expect(energizeLine(undefined, STOPS)).toMatch(/stays unlit/);
    expect(removeEnergize(lit, 'cable').tracks).toEqual({});
  });

  it('refuses a path without a name', () => {
    expect(pathsProblem(wired())).toBeNull();
    expect(pathsProblem(setPath(wired(), 'cable', { name: ' ' }))).toMatch(/name/);
  });
});
