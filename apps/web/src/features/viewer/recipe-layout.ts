/**
 * Where a recipe's prop target sits by the scene layout, by the rule the
 * bake follows: when the layout places both the prop and the model, the
 * target is the prop's middle where the layout puts it, seen from where it
 * puts the model; otherwise the prop takes the preview's slot in front.
 */
import * as THREE from 'three';
import type { LayoutView } from '@/ipc/client';
import { fromLayout, type PlainPlacement } from '@/state/layout';

export type Spot = readonly [number, number, number];

const deg = THREE.MathUtils.degToRad;

/** A placement's matrix, composed as the layout editor and the core compose it. */
function matrixOf(p: PlainPlacement): THREE.Matrix4 {
  const [rx, ry, rz] = p.rotation;
  const turn = new THREE.Quaternion().setFromEuler(
    new THREE.Euler(deg(rx), deg(ry), deg(rz), 'XYZ')
  );
  const scale = new THREE.Vector3().setScalar(p.scale);
  return new THREE.Matrix4().compose(new THREE.Vector3(...p.position), turn, scale);
}

/**
 * The spot of `prop` in the frame of model `asset` (its own glTF frame, from
 * where it stands), or null when the layout does not place both, so the
 * bake and the preview use the slot instead. The middle is the prop's
 * bounds as the layout measures them.
 */
export function layoutSpot(view: LayoutView, asset: string, prop: string): Spot | null {
  const placed = fromLayout(view.layout).models;
  const at = placed[prop];
  const model = placed[asset];
  if (!at || !model) return null;
  const found = view.models.find(m => m.asset === prop);
  const middle = new THREE.Vector3();
  if (found?.min && found.max) {
    const [a, b] = [found.min, found.max];
    middle.set(
      ((a[0] ?? 0) + (b[0] ?? 0)) / 2,
      ((a[1] ?? 0) + (b[1] ?? 0)) / 2,
      ((a[2] ?? 0) + (b[2] ?? 0)) / 2
    );
  }
  const local = middle.applyMatrix4(matrixOf(at)).applyMatrix4(matrixOf(model).invert());
  return [local.x, local.y, local.z];
}
