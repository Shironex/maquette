/**
 * A model's parts in a preview: each mesh node is a part, by the name the
 * export gives it. The parts look gives each one its own colour, draws a
 * hidden one as a ghost and lifts the selected ones; explode pulls them
 * apart from the model's middle, to see where each one ends. Plain
 * three.js, so the Inbox, the Parts tab and Compare share it.
 */
import * as THREE from 'three';
import type { Originals } from './materials';

/** Tells the parts code which group is the loaded file's root. */
export const ROOT_FLAG = 'maquetteRoot';

/**
 * Part colours: one hue each, told apart by lightness too, readable on the
 * dark studio floor.
 */
export const PART_COLOURS: readonly string[] = [
  '#e0a458',
  '#6fa8dc',
  '#93c47d',
  '#c27ba0',
  '#f6d06f',
  '#76c5c0',
  '#d9776b',
  '#b4a7d6',
];

export interface PartLook {
  /** Part names drawn lifted. */
  selected?: readonly string[];
  /** Part names drawn as a ghost. */
  hidden?: readonly string[];
}

const isMesh = (o: THREE.Object3D): o is THREE.Mesh => (o as THREE.Mesh).isMesh === true;

/**
 * The object that stands for `mesh`'s part: the mesh, or the group a node
 * with several materials loads as.
 */
export function partRoot(mesh: THREE.Object3D): THREE.Object3D {
  const parent = mesh.parent;
  if (
    parent &&
    parent.type === 'Group' &&
    !parent.userData[ROOT_FLAG] &&
    parent.children.length > 1 &&
    parent.children.every(isMesh)
  ) {
    return parent;
  }
  return mesh;
}

/** The part name of `mesh`, as the model file has it. */
export const partOf = (mesh: THREE.Object3D): string => partRoot(mesh).name;

/** The colour of the `index`th part. */
export const partColour = (index: number): string =>
  PART_COLOURS[index % PART_COLOURS.length] ?? PART_COLOURS[0]!;

/** Every part name of `originals`, in the order the file has them. */
export function partNames(originals: Originals): string[] {
  const names: string[] = [];
  for (const mesh of originals.keys()) {
    const name = partOf(mesh);
    if (!names.includes(name)) names.push(name);
  }
  return names;
}

/** Dresses each part in its colour; returns the undo. */
export function applyParts(originals: Originals, look: PartLook = {}): () => void {
  const names = partNames(originals);
  const made = new Map<string, THREE.Material>();
  const selected = new Set(look.selected ?? []);
  const hidden = new Set(look.hidden ?? []);
  const materialFor = (name: string): THREE.Material => {
    const known = made.get(name);
    if (known) return known;
    const colour = partColour(names.indexOf(name));
    const ghost = hidden.has(name);
    const material = new THREE.MeshStandardMaterial({
      color: colour,
      roughness: 0.8,
      transparent: ghost,
      opacity: ghost ? 0.16 : 1,
      depthWrite: !ghost,
      emissive: selected.has(name) ? colour : '#000000',
      emissiveIntensity: selected.has(name) ? 0.45 : 0,
    });
    made.set(name, material);
    return material;
  };
  for (const mesh of originals.keys()) {
    const name = partOf(mesh);
    mesh.material = materialFor(name);
    mesh.castShadow = !hidden.has(name);
  }
  return () => {
    for (const [mesh, original] of originals) mesh.material = original;
    made.forEach(m => m.dispose());
  };
}

/**
 * Pulls each part of `root` out from the model's middle by `amount` (0 to
 * 1) of the model's size, along the line from the middle to the part's
 * own. Returns the undo.
 */
export function explode(root: THREE.Object3D, amount: number): () => void {
  if (amount <= 0) return () => undefined;
  root.updateWorldMatrix(true, true);
  const whole = new THREE.Box3().setFromObject(root);
  const middle = whole.getCenter(new THREE.Vector3());
  const reach = whole.getSize(new THREE.Vector3()).length() * amount;
  const parts = new Set<THREE.Object3D>();
  root.traverse(o => {
    if (isMesh(o)) parts.add(partRoot(o));
  });
  const moved: [THREE.Object3D, THREE.Vector3][] = [];
  for (const part of parts) {
    const centre = new THREE.Box3().setFromObject(part).getCenter(new THREE.Vector3());
    const away = centre.clone().sub(middle);
    if (away.lengthSq() < 1e-12) continue;
    const target = centre.clone().add(away.normalize().multiplyScalar(reach));
    const parent = part.parent;
    const from = parent ? parent.worldToLocal(centre.clone()) : centre.clone();
    const to = parent ? parent.worldToLocal(target) : target;
    moved.push([part, part.position.clone()]);
    part.position.add(to.sub(from));
  }
  return () => {
    for (const [part, position] of moved) part.position.copy(position);
  };
}
