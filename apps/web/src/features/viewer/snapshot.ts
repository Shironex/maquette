/**
 * The fallback thumbnail: when Blender cannot render one (it is missing, or
 * the file uses KTX2 textures Blender cannot read), the model is drawn once
 * offscreen in the same studio as the preview and saved as a PNG.
 */
import * as THREE from 'three';
import { BACKDROP, castShadows, frameCamera, gltfLoader, standOnFloor, studioRig } from './studio';

export const SNAPSHOT_SIZE = 256;

function dispose(root: THREE.Object3D): void {
  root.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.geometry.dispose();
    const list = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const material of list) {
      for (const value of Object.values(material)) {
        if (value instanceof THREE.Texture) value.dispose();
      }
      material.dispose();
    }
  });
}

/** Renders the model at `url` to PNG bytes, `size` pixels square. */
export async function snapshotModel(url: string, size = SNAPSHOT_SIZE): Promise<Uint8Array> {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    preserveDrawingBuffer: true,
  });
  renderer.setSize(size, size, false);
  // As react-three-fiber sets up the live preview.
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  const scene = new THREE.Scene();
  try {
    const gltf = await gltfLoader(renderer).loadAsync(url);
    scene.background = new THREE.Color(BACKDROP);
    scene.add(gltf.scene);
    const framing = standOnFloor(gltf.scene);
    if (!framing) throw new Error('The model has nothing to draw.');
    castShadows(gltf.scene);
    scene.add(studioRig(framing));
    const camera = new THREE.PerspectiveCamera(30, 1);
    frameCamera(camera, framing);
    renderer.render(scene, camera);
    const blob = await new Promise<Blob | null>(resolve => canvas.toBlob(resolve, 'image/png'));
    if (!blob) throw new Error('The snapshot could not be encoded.');
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    dispose(scene);
    renderer.dispose();
    renderer.forceContextLoss();
  }
}
