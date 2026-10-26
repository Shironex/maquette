import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { buildPath, curveOf, energize } from './path-mesh';
import { TUBE_SIDES, tubeSegments, type ScenePath } from './paths';

const cable = (curve: ScenePath['curve']): ScenePath => ({
  id: 'main-feed',
  name: 'Main feed',
  curve,
  points: [
    [0, 0.9, 0],
    [1, 0.9, 0],
    [1, 1.9, 0],
  ],
  start: null,
  end: null,
  width: 0.05,
  color: '#f2c230',
  glow: '#ffe08a',
});

describe('a built path', () => {
  it('is one tube along the points, its U running from the start to the end', () => {
    const mesh = buildPath(cable('polyline'));
    const segments = tubeSegments(cable('polyline'));
    const geometry = mesh.geometry;
    expect(geometry.attributes.position!.count).toBe((segments + 1) * (TUBE_SIDES + 1));
    const uv = geometry.attributes.uv!;
    expect(uv.getX(0)).toBe(0);
    expect(uv.getX(uv.count - 1)).toBe(1);
    const box = new THREE.Box3().setFromBufferAttribute(
      geometry.attributes.position as THREE.BufferAttribute
    );
    expect(box.min.y).toBeCloseTo(0.875, 2);
    expect(box.max.y).toBeCloseTo(1.9, 1);
    expect(mesh.name).toBe('path-main-feed');
  });

  it('runs straight for a polyline and bends through every point for catmull-rom', () => {
    const straight = curveOf(cable('polyline'));
    expect(straight.getPointAt(0.25).y).toBeCloseTo(0.9);
    const smooth = curveOf(cable('catmull-rom'));
    expect(smooth.getPoint(1).distanceTo(new THREE.Vector3(1, 1.9, 0))).toBeLessThan(1e-6);
    expect(smooth.getLength()).toBeGreaterThan(1.9);
  });

  it('lights by its amount uniform and starts dark', () => {
    const mesh = buildPath(cable('catmull-rom'));
    expect(mesh.material.uniforms.uAmount!.value).toBe(0);
    energize(mesh, 0.4);
    expect(mesh.material.uniforms.uAmount!.value).toBe(0.4);
    expect(mesh.material.fragmentShader).toContain('uAmount');
  });
});
