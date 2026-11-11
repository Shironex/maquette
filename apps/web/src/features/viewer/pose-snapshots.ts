/**
 * Stills of one clip at a few frames, for a review's pose sheet: the model is
 * loaded once offscreen in the preview's studio, posed at each frame by the
 * clip's own keys and drawn to a small JPEG, one after another on one
 * renderer, so a sheet of eight costs one WebGL context.
 */
import * as THREE from 'three';
import { disposeObject } from './dispose';
import { BACKDROP, castShadows, frameCamera, gltfLoader, standOnFloor, studioRig } from './studio';

export const STILL_SIZE = 160;

/**
 * Data URLs of the model at `url` playing `clip`, at each of `frames` (keys
 * 0 to `last`, as the clip library counts them), `size` pixels square.
 */
export async function poseStills(
  url: string,
  clip: string,
  frames: readonly number[],
  last: number,
  size = STILL_SIZE
): Promise<string[]> {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    preserveDrawingBuffer: true,
  });
  renderer.setSize(size, size, false);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  const scene = new THREE.Scene();
  try {
    const gltf = await gltfLoader(renderer).loadAsync(url);
    scene.background = new THREE.Color(BACKDROP);
    scene.add(gltf.scene);
    const animation = gltf.animations.find(a => a.name === clip) ?? null;
    const mixer = new THREE.AnimationMixer(gltf.scene);
    if (animation) {
      // Held at its end rather than looped back to the first frame.
      const action = mixer.clipAction(animation).setLoop(THREE.LoopOnce, 1);
      action.clampWhenFinished = true;
      action.play();
    }
    mixer.setTime(0);
    const framing = standOnFloor(gltf.scene);
    if (!framing) throw new Error('The model has nothing to draw.');
    castShadows(gltf.scene);
    scene.add(studioRig(framing));
    const camera = new THREE.PerspectiveCamera(30, 1);
    frameCamera(camera, framing);
    const length = animation?.duration ?? 0;
    return frames.map(frame => {
      mixer.setTime(last > 0 ? (Math.min(frame, last) / last) * length : 0);
      renderer.render(scene, camera);
      return canvas.toDataURL('image/jpeg', 0.85);
    });
  } finally {
    disposeObject(scene);
    renderer.dispose();
    renderer.forceContextLoss();
  }
}
