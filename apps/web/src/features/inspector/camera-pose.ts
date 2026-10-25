/** Reading and setting where an orbiting camera stands, shared by the Inspector's canvases. */
import type * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { CameraPose, Vec3 } from '@/state/inspector';

export function poseOf(camera: THREE.PerspectiveCamera, target: THREE.Vector3): CameraPose {
  return {
    position: camera.position.toArray() as Vec3,
    target: target.toArray() as Vec3,
    fov: camera.fov,
  };
}

export function setPose(
  camera: THREE.PerspectiveCamera,
  controls: OrbitControls,
  pose: CameraPose
): void {
  camera.position.set(...pose.position);
  controls.target.set(...pose.target);
  if (camera.fov !== pose.fov) {
    camera.fov = pose.fov;
    camera.updateProjectionMatrix();
  }
  camera.lookAt(controls.target);
}
