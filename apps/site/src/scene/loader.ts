/**
 * Loads the shipped GLBs the way the Maquette app does: meshopt geometry and
 * KTX2 textures through three's Basis transcoder, served at /basis/ (see
 * build/basis-transcoder.ts). Each file loads once; entries that reuse a
 * file get their own clone. Teardown frees the landed files and aborts the
 * fetches still running.
 */
import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { KTX2Loader } from 'three/examples/jsm/loaders/KTX2Loader.js';
import { MeshoptDecoder } from 'three/examples/jsm/libs/meshopt_decoder.module.js';
import { disposeFile, fileGeometries } from './dispose';
import { modelUrl } from './manifest';

let ktx2: KTX2Loader | null = null;
let gltf: GLTFLoader | null = null;
let manager: THREE.LoadingManager | null = null;
const files = new Map<string, Promise<GLTF>>();
/** The files that finished loading since the last teardown. */
const landed = new Map<string, GLTF>();

function loader(renderer: THREE.WebGLRenderer): GLTFLoader {
  if (!gltf) {
    manager = new THREE.LoadingManager();
    ktx2 = new KTX2Loader(manager).setTranscoderPath('/basis/').detectSupport(renderer);
    gltf = new GLTFLoader(manager).setMeshoptDecoder(MeshoptDecoder).setKTX2Loader(ktx2);
  }
  return gltf;
}

export function loadFile(renderer: THREE.WebGLRenderer, file: string): Promise<GLTF> {
  let pending = files.get(file);
  if (!pending) {
    const loading = loader(renderer).loadAsync(modelUrl(file));
    // A file that lands after a teardown is not kept.
    void loading.then(
      model => {
        if (files.get(file) === loading) landed.set(file, model);
      },
      () => undefined
    );
    files.set(file, loading);
    pending = loading;
  }
  return pending;
}

/**
 * Loads `order` a few files at a time, in order, and reports each one as it
 * lands, so the first stop's models show up before the rest are fetched.
 * Once `live` turns false it starts no more files and reports nothing.
 */
export async function loadInOrder(
  renderer: THREE.WebGLRenderer,
  order: readonly string[],
  onLoaded: (file: string, model: GLTF) => void,
  onError: (file: string, error: unknown) => void,
  live: () => boolean = () => true,
  concurrency = 3
): Promise<void> {
  let next = 0;
  const worker = async () => {
    while (next < order.length && live()) {
      const file = order[next++];
      if (!file) continue;
      try {
        const model = await loadFile(renderer, file);
        if (live()) onLoaded(file, model);
      } catch (error) {
        if (live()) onError(file, error);
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(concurrency, order.length) }, worker));
}

/**
 * Frees every landed file once (geometries, textures, materials) and returns
 * their geometries, which the built clones share.
 */
export function disposeFiles(): Set<THREE.BufferGeometry> {
  const shared = new Set<THREE.BufferGeometry>();
  for (const model of landed.values()) {
    for (const geometry of fileGeometries(model.scene)) shared.add(geometry);
    disposeFile(model.scene);
  }
  landed.clear();
  return shared;
}

/** Aborts the fetches still running and drops the loaders and the file cache. */
export function disposeLoaders(): void {
  manager?.abort();
  ktx2?.dispose();
  manager = null;
  ktx2 = null;
  gltf = null;
  files.clear();
  landed.clear();
}
