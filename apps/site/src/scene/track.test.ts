import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { createTrack } from './track';

const point = new THREE.Vector3();

describe('createTrack', () => {
  const points = [
    new THREE.Vector3(0, 0, 0),
    new THREE.Vector3(1, 0, 0),
    new THREE.Vector3(1.05, 0, 0.02),
    new THREE.Vector3(2, 1, 0),
  ];
  const track = createTrack(points);

  it('measures the distance to each of its points', () => {
    expect(track.lengths[0]).toBe(0);
    expect(track.lengths[1]).toBeCloseTo(1, 1);
    for (let i = 1; i < track.lengths.length; i++) {
      expect(track.lengths[i]).toBeGreaterThan(track.lengths[i - 1] ?? 0);
    }
  });

  it('lands on each point at its distance', () => {
    points.forEach((p, i) => {
      track.pointAt(track.lengths[i] ?? 0, point);
      expect(point.distanceTo(p)).toBeLessThan(1e-6);
    });
  });

  it('clamps to its ends', () => {
    track.pointAt(-1, point);
    expect(point.distanceTo(points[0] as THREE.Vector3)).toBeLessThan(1e-6);
    track.pointAt(99, point);
    expect(point.distanceTo(points[3] as THREE.Vector3)).toBeLessThan(1e-6);
  });

  it('moves evenly with distance', () => {
    const a = track.pointAt(0.2, new THREE.Vector3());
    const b = track.pointAt(0.3, new THREE.Vector3());
    expect(a.distanceTo(b)).toBeCloseTo(0.1, 2);
  });
});
