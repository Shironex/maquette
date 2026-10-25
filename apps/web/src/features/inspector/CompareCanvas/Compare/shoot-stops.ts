import type * as THREE from 'three';
import type { Stop } from '@/state/inspector';

const SHOT = { width: 320, height: 180 };

/**
 * Renders every stop without a poster into a small JPEG, inside one frame:
 * the scene from each stop's camera, cover cropped to 16:9.
 */
export function shootStops(
  gl: THREE.WebGLRenderer,
  camera: THREE.PerspectiveCamera,
  size: { width: number; height: number },
  scene: THREE.Scene,
  stops: Stop[]
): Record<number, string> | null {
  const shots: Record<number, string> = {};
  const canvas = document.createElement('canvas');
  canvas.width = SHOT.width;
  canvas.height = SHOT.height;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  const shot = camera.clone();
  const el = gl.domElement;
  shot.aspect = el.width / el.height;
  gl.setScissorTest(false);
  gl.setViewport(0, 0, size.width, size.height);
  stops.forEach((stop, i) => {
    if (stop.poster) return;
    shot.fov = stop.fov;
    shot.position.set(...stop.position);
    shot.lookAt(...stop.target);
    shot.updateProjectionMatrix();
    gl.render(scene, shot);
    // Cover crop to 16:9.
    const want = SHOT.width / SHOT.height;
    const sw = Math.min(el.width, el.height * want);
    const sh = sw / want;
    ctx.drawImage(
      el,
      (el.width - sw) / 2,
      (el.height - sh) / 2,
      sw,
      sh,
      0,
      0,
      SHOT.width,
      SHOT.height
    );
    shots[i] = canvas.toDataURL('image/jpeg', 0.84);
  });
  return shots;
}
