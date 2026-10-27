import * as THREE from 'three';
import { materialTextures } from './dispose';

const nextFrame = () => new Promise<void>(done => requestAnimationFrame(() => done()));

/**
 * Gets a model ready before it joins the scene: its shaders compile in
 * parallel where the driver allows it and its textures upload, one model per
 * frame, so no single frame does every model's work at once.
 */
export async function warm(
  gl: THREE.WebGLRenderer,
  root: THREE.Object3D,
  camera: THREE.Camera,
  scene: THREE.Scene
): Promise<void> {
  await gl.compileAsync(root, camera, scene);
  await nextFrame();
  root.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const materials: THREE.Material[] = Array.isArray(object.material)
      ? object.material
      : [object.material];
    for (const material of materials) {
      for (const texture of materialTextures(material)) gl.initTexture(texture);
    }
  });
}
