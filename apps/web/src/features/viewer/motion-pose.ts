/**
 * Posing a loaded model for the Motion view: playing one of its clips at a
 * given time, moving named parts by offsets from their rest pose (object
 * states), and the path its root travels over a clip. Plain three.js.
 */
import * as THREE from 'three';

export type Vec3 = [number, number, number];

/** An offset from a part's rest pose: metres, and degrees X then Y then Z. */
export interface PartOffset {
  position: Vec3;
  rotation: Vec3;
}

interface Rest {
  position: THREE.Vector3;
  rotation: THREE.Euler;
}

/** The name the file gave a node: three's loader cleans spaces out of `name`. */
export function fileName(object: THREE.Object3D): string {
  const original = (object.userData as { name?: unknown }).name;
  return typeof original === 'string' ? original : object.name;
}

/** Every named node by its file name, with its rest transform. */
export function restPoses(
  scene: THREE.Object3D
): Map<string, { node: THREE.Object3D; rest: Rest }> {
  const out = new Map<string, { node: THREE.Object3D; rest: Rest }>();
  scene.traverse(node => {
    const name = fileName(node);
    if (!name || out.has(name)) return;
    out.set(name, {
      node,
      rest: { position: node.position.clone(), rotation: node.rotation.clone() },
    });
  });
  return out;
}

const DEG = Math.PI / 180;

/** Puts every part back at rest, then moves the ones `offsets` names. */
export function applyOffsets(
  parts: Map<string, { node: THREE.Object3D; rest: Rest }>,
  offsets: Record<string, PartOffset> | null
): void {
  for (const [name, { node, rest }] of parts) {
    const offset = offsets?.[name];
    node.position.copy(rest.position);
    node.rotation.copy(rest.rotation);
    if (!offset) continue;
    node.position.x += offset.position[0];
    node.position.y += offset.position[1];
    node.position.z += offset.position[2];
    node.rotation.set(
      rest.rotation.x + offset.rotation[0] * DEG,
      rest.rotation.y + offset.rotation[1] * DEG,
      rest.rotation.z + offset.rotation[2] * DEG,
      rest.rotation.order
    );
  }
}

/** A mixer playing `clip` alone, held so the caller sets its time. */
export function clipPlayer(scene: THREE.Object3D, clip: THREE.AnimationClip) {
  const mixer = new THREE.AnimationMixer(scene);
  const action = mixer.clipAction(clip);
  action.play();
  return {
    mixer,
    /** Poses the model at `time` seconds into the clip. */
    seek(time: number) {
      action.time = Math.min(Math.max(time, 0), clip.duration);
      mixer.update(0);
    },
    dispose() {
      action.stop();
      mixer.stopAllAction();
      mixer.uncacheRoot(scene);
    },
  };
}

/** The translated node nearest the root among the clip's tracks: what walks. */
export function rootOf(scene: THREE.Object3D, clip: THREE.AnimationClip): THREE.Object3D | null {
  let best: { node: THREE.Object3D; depth: number } | null = null;
  for (const track of clip.tracks) {
    if (!track.name.endsWith('.position')) continue;
    const target = track.name.slice(0, -'.position'.length);
    const node = scene.getObjectByName(target);
    if (!node) continue;
    let depth = 0;
    for (let p = node.parent; p && p !== scene; p = p.parent) depth += 1;
    if (!best || depth < best.depth) best = { node, depth };
  }
  return best?.node ?? null;
}

/**
 * Where the root stands on the ground over the clip, `samples` points from
 * start to end, in world space. Leaves the model at the clip's start.
 */
export function rootTrail(
  player: ReturnType<typeof clipPlayer>,
  scene: THREE.Object3D,
  root: THREE.Object3D,
  duration: number,
  samples = 24
): THREE.Vector3[] {
  const points: THREE.Vector3[] = [];
  for (let i = 0; i <= samples; i += 1) {
    player.seek((duration * i) / samples);
    scene.updateMatrixWorld(true);
    const at = root.getWorldPosition(new THREE.Vector3());
    points.push(new THREE.Vector3(at.x, 0, at.z));
  }
  player.seek(0);
  return points;
}

/** A dashed line along `points`, a little above the floor. */
export function trailLine(points: THREE.Vector3[], color: string, lift: number): THREE.Line {
  const geometry = new THREE.BufferGeometry().setFromPoints(
    points.map(p => new THREE.Vector3(p.x, p.y + lift, p.z))
  );
  const material = new THREE.LineDashedMaterial({
    color,
    dashSize: lift * 20,
    gapSize: lift * 14,
    transparent: true,
    opacity: 0.8,
  });
  const line = new THREE.Line(geometry, material);
  line.computeLineDistances();
  line.renderOrder = 5;
  return line;
}
