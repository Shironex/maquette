/**
 * Builds one manifest entry into scene objects: an outer group at the
 * entry's position (the pivot it spins about) holding the rotated, scaled
 * model, offset so its anchor lands on that position. A model exported in
 * place is only moved back by the point it already stands at.
 */
import * as THREE from 'three';
import { withBase } from '../lib/with-base';
import type { ModelEntry } from './manifest';
import { prepareModel } from './prepare-model';
import { addSweep, type Sweep } from './sweep';

export interface BuiltModel {
  entry: ModelEntry;
  root: THREE.Group;
  /** The model itself inside `root`, as the file has it. */
  model: THREE.Object3D;
  sweep: Sweep | null;
}

const DEG = Math.PI / 180;
const textures = new Map<string, THREE.Texture>();

function screenTexture(path: string): THREE.Texture {
  let texture = textures.get(path);
  if (!texture) {
    texture = new THREE.TextureLoader().load(withBase(path));
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = 4;
    textures.set(path, texture);
  }
  return texture;
}

/** Frees the screen textures the built models share. */
export function disposeScreens(): void {
  for (const texture of textures.values()) texture.dispose();
  textures.clear();
}

export function buildModel(entry: ModelEntry, scene: THREE.Object3D): BuiltModel {
  const inner = scene.clone(true);
  const [rx, ry, rz] = entry.rotation ?? [0, 0, 0];
  inner.rotation.set(rx * DEG, ry * DEG, rz * DEG);
  inner.scale.setScalar(entry.scale ?? 1);
  prepareModel(inner, entry.material, {
    cast: entry.castShadow ?? false,
    receive: entry.receiveShadow ?? false,
  });

  if (entry.inPlace) {
    const [x, y, z] = entry.inPlace;
    inner.position.set(-x, -y, -z);
  } else if (entry.anchor === 'bottom-center') {
    inner.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(inner);
    const center = box.getCenter(new THREE.Vector3());
    inner.position.set(-center.x, -box.min.y, -center.z);
  }

  const root = new THREE.Group();
  root.name = entry.id;
  root.position.set(...entry.position);
  root.add(inner);

  if (entry.screen) {
    const { texture, size, position, rotation } = entry.screen;
    const screen = new THREE.Mesh(
      new THREE.PlaneGeometry(size[0], size[1]),
      new THREE.MeshBasicMaterial({
        map: screenTexture(texture),
        color: new THREE.Color(0.82, 0.82, 0.82),
        toneMapped: false,
      })
    );
    screen.name = `${entry.id}-screen`;
    screen.position.set(...position);
    screen.rotation.set(rotation[0] * DEG, rotation[1] * DEG, rotation[2] * DEG);
    // The decal is placed in the model's own frame, so it turns with the model.
    const holder = new THREE.Group();
    holder.rotation.y = ry * DEG;
    holder.add(screen);
    root.add(holder);
  }

  const sweep = entry.sweepAt ? addSweep(inner) : null;
  return { entry, root, model: inner, sweep };
}
