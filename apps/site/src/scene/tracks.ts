/**
 * Moves a built model's parts by the scroll timeline (./timeline.ts): its
 * state poses and its explode track. Parts are the model's glTF nodes by
 * the names in the file; three's loader keeps those in `userData.name`.
 */
import * as THREE from 'three';
import type { BuiltModel } from './build';
import { explodeAt, stateAnimator, type ExplodeTrack, type ObjectStates } from './timeline';

const DEG = Math.PI / 180;

export interface TimelineTracks {
  /** Camera stop ids in tour order, as far as the site's stops go. */
  stopIds: readonly string[];
  states: Record<string, ObjectStates>;
  explode: Record<string, ExplodeTrack>;
}

export interface Tracks {
  /** Poses the parts for tour progress and clock time; true while a move still plays. */
  apply(progress: number, now: number): boolean;
}

interface Part {
  node: THREE.Object3D;
  position: THREE.Vector3;
  rotation: THREE.Euler;
  /** The part's own move per metre of spread, in its parent's space. */
  out: THREE.Vector3 | null;
}

const fileName = (node: THREE.Object3D): string | null =>
  typeof node.userData.name === 'string' ? node.userData.name : null;

const isMesh = (node: THREE.Object3D) => (node as THREE.Mesh).isMesh === true;

/**
 * The nodes that draw a mesh (a node with several primitives holds them as
 * unnamed children), the top-most ones only so nothing moves twice.
 */
export function meshParts(model: THREE.Object3D): THREE.Object3D[] {
  const found: THREE.Object3D[] = [];
  model.traverse(node => {
    if (node === model || !fileName(node)) return;
    if (isMesh(node) || node.children.some(c => isMesh(c) && !fileName(c))) found.push(node);
  });
  const parts = new Set(found);
  return found.filter(node => {
    for (let p = node.parent; p && p !== model; p = p.parent) if (parts.has(p)) return false;
    return true;
  });
}

/** Each part's way out from the model's centre, one metre long, in its parent's space. */
function outward(model: THREE.Object3D, node: THREE.Object3D): THREE.Vector3 {
  const centre = new THREE.Box3().setFromObject(model).getCenter(new THREE.Vector3());
  const own = new THREE.Box3().setFromObject(node).getCenter(new THREE.Vector3());
  const way = own.sub(centre);
  if (way.lengthSq() < 1e-10) way.set(0, 1, 0);
  way.normalize();
  const from = node.getWorldPosition(new THREE.Vector3());
  const parent = node.parent ?? model;
  const start = parent.worldToLocal(from.clone());
  return parent.worldToLocal(from.add(way)).sub(start);
}

/** The model's tracks, or null when the timeline does not move it. */
export function rigTracks(built: BuiltModel, timeline: TimelineTracks): Tracks | null {
  const asset = built.entry.id;
  const object = timeline.states[asset];
  const track = timeline.explode[asset];
  const moving = object?.triggers.some(t => t.kind === 'stop' || t.kind === 'scroll');
  if (!moving && !track?.keys.length) return null;
  const model = built.model;
  built.root.updateMatrixWorld(true);

  const parts = new Map<string, Part>();
  const add = (node: THREE.Object3D, name: string, out: THREE.Vector3 | null) => {
    const known = parts.get(name);
    if (known) {
      known.out ??= out;
      return;
    }
    parts.set(name, {
      node,
      position: node.position.clone(),
      rotation: node.rotation.clone(),
      out,
    });
  };
  if (track) for (const node of meshParts(model)) add(node, fileName(node)!, outward(model, node));
  if (object) {
    const named = new Set(object.states.flatMap(s => Object.keys(s.parts)));
    model.traverse(node => {
      const name = fileName(node);
      if (name && named.has(name) && !parts.has(name)) add(node, name, null);
    });
  }

  const states = moving && object ? stateAnimator(object, timeline.stopIds) : null;
  const distance = Math.max(0, track?.distance ?? 0);
  const step = new THREE.Vector3();
  return {
    apply(progress, now) {
      const played = states?.update(progress, now);
      const spread = track ? explodeAt(track, timeline.stopIds, progress) * distance : 0;
      for (const [name, part] of parts) {
        const { node, position, rotation, out } = part;
        const offset = played?.pose[name];
        node.position.copy(position);
        if (offset) node.position.add(step.set(...offset.position));
        if (out && spread > 0) node.position.addScaledVector(out, spread);
        node.rotation.set(
          rotation.x + (offset?.rotation[0] ?? 0) * DEG,
          rotation.y + (offset?.rotation[1] ?? 0) * DEG,
          rotation.z + (offset?.rotation[2] ?? 0) * DEG,
          rotation.order
        );
      }
      return played?.moving ?? false;
    },
  };
}
