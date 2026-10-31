import { describe, expect, it } from 'vitest';
import type { ScenePath } from '@/ipc/client';
import type { Vec3 } from './inspector';
import { addStroke, fitStroke, thinStroke } from './path-stroke';
import { MAX_POINTS, type PathsDraft } from './paths';

const path = (count: number): ScenePath => ({
  id: 'cable',
  name: 'Cable',
  curve: 'catmull-rom',
  points: Array.from({ length: count }, (_, i) => [i, 0, 0] as Vec3),
  start: null,
  end: null,
  width: 0.03,
  color: '#f2c230',
  glow: '#ffe08a',
});

const draft = (count: number): PathsDraft => ({ paths: [path(count)], tracks: {} });

describe('a stroke over the models', () => {
  it('keeps only the samples that hold its shape', () => {
    const straight: Vec3[] = Array.from({ length: 50 }, (_, i) => [i * 0.01, 0, 0]);
    expect(thinStroke(straight)).toEqual([straight[0], straight[49]]);
    const corner: Vec3[] = [...straight, [0.49, 0, 0.3]];
    expect(thinStroke(corner)).toEqual([straight[0], straight[49], [0.49, 0, 0.3]]);
  });

  it('fits what the path has room for, keeping both ends', () => {
    const points: Vec3[] = Array.from({ length: 9 }, (_, i) => [i, 0, 0]);
    expect(fitStroke(points, 3)).toEqual([points[0], points[4], points[8]]);
    expect(fitStroke(points, 0)).toEqual([]);
    expect(fitStroke(points, 20)).toEqual(points);
  });

  it('adds after the picked point, in order, and answers with its last', () => {
    const stroke: Vec3[] = [
      [5, 1, 0],
      [6, 2, 0],
      [7, 1, 0],
    ];
    const { draft: next, index } = addStroke(draft(3), 'cable', stroke, 0, false);
    expect(index).toBe(3);
    expect(next.paths[0]!.points.slice(0, 5)).toEqual([[0, 0, 0], ...stroke, [1, 0, 0]]);
  });

  it('snaps when the magnet is on, and stops at the point limit', () => {
    const snapped = addStroke(draft(2), 'cable', [[0.512, 0, 0.0]], null, true);
    expect(snapped.draft.paths[0]!.points.at(-1)).toEqual([0.5, 0, 0]);
    const full = addStroke(draft(MAX_POINTS), 'cable', [[9, 9, 9]], null, false);
    expect(full.index).toBe(-1);
  });
});
