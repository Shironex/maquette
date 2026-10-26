/**
 * A path as the editor draws it: the same curve and tube the site builds
 * (apps/site path-mesh.ts), lit flat in its glow colour when selected.
 */
import * as THREE from 'three';
import type { ScenePath } from '@/ipc/client';
import { TUBE_SIDES, pointsOf, tubeTris } from '@/state/paths';
import { num } from '@/state/inspector';

/** The curve the tube follows: straight runs, or a smooth curve through every point. */
export function curveOf(path: ScenePath): THREE.Curve<THREE.Vector3> {
  const points = pointsOf(path).map(p => new THREE.Vector3(...p));
  if (path.curve === 'catmull-rom') return new THREE.CatmullRomCurve3(points, false, 'centripetal');
  const runs = new THREE.CurvePath<THREE.Vector3>();
  for (let i = 1; i < points.length; i += 1) {
    runs.add(new THREE.LineCurve3(points[i - 1], points[i]));
  }
  return runs;
}

/** The path's tube, `userData.path` its id; null for a path with fewer than two points. */
export function pathTube(path: ScenePath, selected: boolean): THREE.Mesh | null {
  if (path.points.length < 2) return null;
  const rings = tubeTris(path) / (TUBE_SIDES * 2);
  const geometry = new THREE.TubeGeometry(
    curveOf(path),
    rings,
    num(path.width) / 2,
    TUBE_SIDES,
    false
  );
  const material = new THREE.MeshStandardMaterial({
    color: path.color,
    emissive: selected ? path.glow : '#000000',
    emissiveIntensity: selected ? 0.6 : 0,
    roughness: 0.6,
  });
  const mesh = new THREE.Mesh(geometry, material);
  mesh.name = `path-${path.id}`;
  mesh.userData.path = path.id;
  return mesh;
}
