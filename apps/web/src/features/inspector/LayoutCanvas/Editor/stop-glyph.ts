import * as THREE from 'three';
import type { PlainStop } from '@/state/layout';
import { ASPECT } from './fit-fov';
import { deg } from './placement';

/** How long a stop's frustum is drawn, as a share of the scene's radius. */
const FRUSTUM = 0.07;

/** A stop drawn as a small camera: a frustum toward its target and a dot there. */
export function stopGlyph(
  stop: PlainStop,
  color: string,
  first: boolean,
  span: number
): THREE.Group {
  const length = Math.max(0.06, span * FRUSTUM);
  const group = new THREE.Group();
  group.userData.stop = stop.id;
  const from = new THREE.Vector3(...stop.position);
  const to = new THREE.Vector3(...stop.target);
  const forward = to.clone().sub(from).normalize();
  const right = new THREE.Vector3().crossVectors(forward, new THREE.Vector3(0, 1, 0));
  if (right.lengthSq() < 1e-6) right.set(1, 0, 0);
  right.normalize();
  const up = new THREE.Vector3().crossVectors(right, forward).normalize();
  const h = Math.tan(deg(stop.fov) / 2) * length;
  const w = h * ASPECT;
  const center = forward.clone().multiplyScalar(length);
  const corners = [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ].map(([x, y]) =>
    center
      .clone()
      .addScaledVector(right, x! * w)
      .addScaledVector(up, y! * h)
  );
  const points: THREE.Vector3[] = [];
  for (let i = 0; i < 4; i++) {
    points.push(new THREE.Vector3(), corners[i]!, corners[i]!, corners[(i + 1) % 4]!);
  }
  // The little roof marks which way is up.
  const roof = center.clone().addScaledVector(up, h * 1.6);
  points.push(corners[2]!, roof, roof, corners[3]!);
  const frustum = new THREE.LineSegments(
    new THREE.BufferGeometry().setFromPoints(points),
    new THREE.LineBasicMaterial({ color, depthTest: false, transparent: true })
  );
  frustum.renderOrder = 3;
  const toward = to.clone().sub(from);
  const sight = new THREE.Line(
    new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(), toward]),
    new THREE.LineDashedMaterial({
      color,
      dashSize: length * 0.16,
      gapSize: length * 0.12,
      depthTest: false,
      transparent: true,
      opacity: 0.6,
    })
  );
  sight.computeLineDistances();
  sight.renderOrder = 3;
  const dot = new THREE.Mesh(
    new THREE.SphereGeometry(length * (first ? 0.08 : 0.06), 12, 8),
    new THREE.MeshBasicMaterial({ color, depthTest: false, transparent: true })
  );
  dot.position.copy(toward);
  dot.renderOrder = 3;
  // An invisible box at the camera to click and to hang the gizmo on.
  const body = new THREE.Mesh(
    new THREE.BoxGeometry(length * 0.45, length * 0.45, length * 0.45),
    new THREE.MeshBasicMaterial({ visible: false })
  );
  body.userData.stop = stop.id;
  body.name = 'stop-body';
  // The group stands at the camera, so the gizmo moves the whole glyph.
  group.position.copy(from);
  group.add(frustum, sight, dot, body);
  return group;
}
