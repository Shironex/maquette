import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { buildModel } from './build';
import { disposeBuilt, disposeFile, fileGeometries } from './dispose';
import type { ModelEntry } from './manifest';

/**
 * What a WebGLRenderer keeps per resource, booked the way three's WebGLInfo
 * books `memory.geometries` and `memory.textures`: counted on first upload,
 * uncounted on the resource's `dispose` event. Node has no WebGL, so this
 * stands in for `renderer.info.memory`; materials stand in for programs.
 */
function fakeRenderer() {
  const memory = { geometries: 0, textures: 0, materials: 0 };
  const disposed = new Map<object, number>();
  const seen = new Set<object>();
  const book = (resource: THREE.EventDispatcher<{ dispose: object }>, key: keyof typeof memory) => {
    if (seen.has(resource)) return;
    seen.add(resource);
    memory[key]++;
    const onDispose = () => {
      memory[key]--;
      disposed.set(resource, (disposed.get(resource) ?? 0) + 1);
      resource.removeEventListener('dispose', onDispose);
      seen.delete(resource);
    };
    resource.addEventListener('dispose', onDispose);
  };
  const render = (root: THREE.Object3D) =>
    root.traverse(child => {
      const mesh = child as THREE.Mesh;
      if (!mesh.isMesh) return;
      book(mesh.geometry, 'geometries');
      const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
      for (const material of materials) {
        book(material, 'materials');
        for (const value of Object.values(material)) {
          if (value instanceof THREE.Texture) book(value, 'textures');
        }
      }
    });
  return { memory, disposed, render };
}

/** A loaded file: two meshes sharing a colour map, one with a light bake. */
function loadedFile() {
  const map = new THREE.Texture();
  const bake = new THREE.Texture();
  bake.channel = 1;
  const scene = new THREE.Group();
  scene.add(
    new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial({ map })),
    new THREE.Mesh(
      new THREE.BoxGeometry(),
      new THREE.MeshStandardMaterial({ map, emissiveMap: bake })
    )
  );
  return { scene, textures: [map, bake] };
}

const entry = (id: string, extra: Partial<ModelEntry> = {}): ModelEntry => ({
  id,
  file: 'file.glb',
  position: [0, 0, 0],
  anchor: 'origin',
  stops: ['hero'],
  ...extra,
});

describe('model teardown', () => {
  it('brings geometries, textures and materials back to the baseline', () => {
    const gpu = fakeRenderer();
    const file = loadedFile();
    // Two entries share the file; one gets the wireframe sweep.
    const built = [
      buildModel(entry('a'), file.scene),
      buildModel(entry('b', { sweepAt: 'hero' }), file.scene),
    ];
    for (const model of built) gpu.render(model.root);
    expect(gpu.memory).toEqual({ geometries: 2, textures: 2, materials: 5 });

    const shared = fileGeometries(file.scene);
    disposeFile(file.scene);
    for (const model of built) disposeBuilt(model.root, shared);

    expect(gpu.memory).toEqual({ geometries: 0, textures: 0, materials: 0 });
    // Each shared texture is freed once, by its file, not once per clone.
    for (const texture of file.textures) expect(gpu.disposed.get(texture)).toBe(1);
  });

  it('leaves the shared textures and geometry alone when one clone goes', () => {
    const gpu = fakeRenderer();
    const file = loadedFile();
    const [kept, gone] = [buildModel(entry('a'), file.scene), buildModel(entry('b'), file.scene)];
    gpu.render(kept.root);
    gpu.render(gone.root);

    disposeBuilt(gone.root, fileGeometries(file.scene));

    expect(gpu.memory).toEqual({ geometries: 2, textures: 2, materials: 2 });
    for (const texture of file.textures) expect(gpu.disposed.has(texture)).toBe(false);
  });

  it('frees geometry a clone owns, such as a screen plane', () => {
    const gpu = fakeRenderer();
    const file = loadedFile();
    const model = buildModel(entry('a'), file.scene);
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.MeshBasicMaterial());
    model.root.add(plane);
    gpu.render(model.root);

    disposeFile(file.scene);
    disposeBuilt(model.root, fileGeometries(file.scene));

    expect(gpu.memory).toEqual({ geometries: 0, textures: 0, materials: 0 });
  });
});
