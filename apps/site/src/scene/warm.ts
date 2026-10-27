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
  // Without the parallel compile extension compileAsync only polls a compile
  // that already blocked, and three warns about the missing extension.
  if (gl.extensions.has('KHR_parallel_shader_compile')) await gl.compileAsync(root, camera, scene);
  else gl.compile(root, camera, scene);
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
