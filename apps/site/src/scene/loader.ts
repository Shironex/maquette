/**
 * Loads the shipped GLBs the way the Maquette app does: meshopt geometry and
 * KTX2 textures through three's Basis transcoder, served at /basis/ (see
 * build/basis-transcoder.ts). Each file loads once; entries that reuse a
 * file get their own clone.
 */
import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { KTX2Loader } from 'three/examples/jsm/loaders/KTX2Loader.js';
import { MeshoptDecoder } from 'three/examples/jsm/libs/meshopt_decoder.module.js';
import { modelUrl } from './manifest';

let ktx2: KTX2Loader | null = null;
let gltf: GLTFLoader | null = null;
const files = new Map<string, Promise<GLTF>>();

function loader(renderer: THREE.WebGLRenderer): GLTFLoader {
  if (!gltf) {
    ktx2 = new KTX2Loader().setTranscoderPath('/basis/').detectSupport(renderer);
    gltf = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).setKTX2Loader(ktx2);
  }
  return gltf;
}

export function loadFile(renderer: THREE.WebGLRenderer, file: string): Promise<GLTF> {
  let pending = files.get(file);
  if (!pending) {
    pending = loader(renderer).loadAsync(modelUrl(file));
    files.set(file, pending);
  }
  return pending;
}

/**
 * Loads `order` a few files at a time, in order, and reports each one as it
 * lands, so the first stop's models show up before the rest are fetched.
 */
export async function loadInOrder(
  renderer: THREE.WebGLRenderer,
  order: readonly string[],
  onLoaded: (file: string, model: GLTF) => void,
  onError: (file: string, error: unknown) => void,
  concurrency = 3
): Promise<void> {
  let next = 0;
  const worker = async () => {
    while (next < order.length) {
      const file = order[next++];
      if (!file) continue;
      try {
        onLoaded(file, await loadFile(renderer, file));
      } catch (error) {
        onError(file, error);
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(concurrency, order.length) }, worker));
}

export function disposeLoaders(): void {
  ktx2?.dispose();
  ktx2 = null;
  gltf = null;
  files.clear();
}
