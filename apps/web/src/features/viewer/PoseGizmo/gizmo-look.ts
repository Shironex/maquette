/**
 * The turn gizmo's three.js objects, made here rather than as JSX props so
 * every material draws over the model (no depth test) and in a set order:
 * a torus per axis sized to the model, and a dot per body bone whose look
 * says picked, keyed or open.
 */
import { useEffect, useMemo } from 'react';
import * as THREE from 'three';

/** The rings' colours, X, Y and Z, as the axis fields' marks. */
export const RING_COLOURS = ['#d9705f', '#8fb17a', '#6f9fd8'] as const;
const DOT = { picked: '#e9c46a', keyed: '#e8e2d6', open: '#e8e2d6' } as const;

const overlay = (color: string, opacity: number) =>
  new THREE.MeshBasicMaterial({ color, depthTest: false, transparent: true, opacity });

/** A ring per axis, `size` the model's height in metres. */
export function useRings(size: number) {
  const rings = useMemo(() => {
    const radius = size * 0.12;
    return RING_COLOURS.map((colour, i) => {
      const mesh = new THREE.Mesh(
        new THREE.TorusGeometry(radius, radius * 0.045, 8, 48),
        overlay(colour, 0.85)
      );
      mesh.name = `turn ring ${'XYZ'[i]}`;
      mesh.renderOrder = 20;
      return mesh;
    });
  }, [size]);
  useEffect(
    () => () => {
      for (const r of rings) {
        r.geometry.dispose();
        (r.material as THREE.Material).dispose();
      }
    },
    [rings]
  );
  return rings;
}

/** Marks ring `active` (being dragged) bolder. */
export function markDragged(rings: readonly THREE.Mesh[], active: number | null) {
  rings.forEach((r, i) => {
    const m = r.material as THREE.MeshBasicMaterial;
    m.opacity = i === active ? 1 : 0.85;
    r.scale.setScalar(i === active ? 1.04 : 1);
  });
}

/** A dot per bone in `dots`, styled for the picked bone and the keyed ones. */
export function useDots(
  dots: readonly string[],
  size: number,
  bone: string,
  keyed: readonly string[]
) {
  const key = dots.join(',');
  const meshes = useMemo(() => {
    const out = new Map<string, THREE.Mesh>();
    for (const name of key.split(',').filter(Boolean)) {
      const mesh = new THREE.Mesh(
        new THREE.SphereGeometry(size * 0.01, 12, 8),
        overlay(DOT.open, 0.45)
      );
      mesh.name = `bone dot ${name}`;
      mesh.renderOrder = 21;
      out.set(name, mesh);
    }
    return out;
  }, [key, size]);
  useEffect(() => {
    for (const [name, mesh] of meshes) {
      const m = mesh.material as THREE.MeshBasicMaterial;
      const picked = name === bone;
      const filled = keyed.includes(name);
      m.color.set(picked ? DOT.picked : filled ? DOT.keyed : DOT.open);
      m.opacity = picked || filled ? 1 : 0.45;
      m.wireframe = !picked && !filled;
      mesh.scale.setScalar(picked ? 1.4 : 1);
    }
  }, [meshes, bone, keyed]);
  useEffect(
    () => () => {
      for (const mesh of meshes.values()) {
        mesh.geometry.dispose();
        (mesh.material as THREE.Material).dispose();
      }
    },
    [meshes]
  );
  return meshes;
}
