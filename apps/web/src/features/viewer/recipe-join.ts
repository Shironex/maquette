/**
 * Joined clips played roughly on a model, frame by frame of the recipe's
 * timeline, as the bake makes them in Blender: each clip's pose at its own
 * frame (a held pose slerped in by its weight), each joined clip lined up
 * with the one before at its seam (hips moved over, and turned to face the
 * same way), the blend window slerping one into the next, the mirror
 * swapping sides across the model's middle, and a held pose breathing on
 * the chest. Round 1's changes go over that, by the timeline's share.
 */
import * as THREE from 'three';
import { BREATHE_SECONDS, type ClipSample, type Timeline } from '@/state/recipe-timeline';

/** How far a held pose breathes at most, radians, before its breathe and weight. */
export const BREATHE_ANGLE = THREE.MathUtils.degToRad(3);

const UP = new THREE.Vector3(0, 1, 0);

/** What the composer needs of a model set up to follow the recipe's clips. */
export interface JoinPoser {
  /** The bones the clips drive, parents first. */
  bones: readonly THREE.Object3D[];
  hips: THREE.Object3D | null;
  /** Each bone's partner on the other side (itself in the middle), for the mirror. */
  partner: (bone: THREE.Object3D) => THREE.Object3D;
  /** The chest and the spine, which breathe while a pose holds. */
  chest: THREE.Object3D | null;
  spine: THREE.Object3D | null;
  /** The way the model faces, world, flat. */
  forward: THREE.Vector3;
  /** Where the model's own origin stands, world. */
  origin: THREE.Vector3;
  /** Puts the model back in its lined up rest. */
  rest: () => void;
  /** Poses the model as clip `clip` stands at frame `at` of its own frames. */
  put: (clip: number, at: number) => void;
}

/** A pose of the driven bones: their local turns and the hips' local place. */
interface Pose {
  turns: THREE.Quaternion[];
  hips: THREE.Vector3 | null;
}

/** How a joined clip is moved to line up with the clip before: a turn about Y through `pivot`, then a move. */
interface LineUp {
  yaw: number;
  pivot: THREE.Vector3;
  move: THREE.Vector3;
}

const NONE: LineUp = { yaw: 0, pivot: new THREE.Vector3(), move: new THREE.Vector3() };

const worldRotation = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());
const worldPosition = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());

/** Turns `bone` so its world rotation is `world`, under its parent as it stands. */
function setWorldRotation(bone: THREE.Object3D, world: THREE.Quaternion) {
  const parent = bone.parent ? worldRotation(bone.parent) : new THREE.Quaternion();
  bone.quaternion.copy(parent.invert().multiply(world));
  bone.updateWorldMatrix(false, true);
}

/** Moves `bone` so its world place is `world`. */
function setWorldPosition(bone: THREE.Object3D, world: THREE.Vector3) {
  bone.position.copy(bone.parent ? bone.parent.worldToLocal(world.clone()) : world);
  bone.updateWorldMatrix(false, true);
}

/** `q` reflected across the plane through the origin at right angles to `normal`: S q S. */
export function reflectRotation(q: THREE.Quaternion, normal: THREE.Vector3): THREE.Quaternion {
  const v = new THREE.Vector3(q.x, q.y, q.z);
  const r = normal
    .clone()
    .multiplyScalar(2 * v.dot(normal))
    .sub(v);
  return new THREE.Quaternion(r.x, r.y, r.z, q.w);
}

/** `p` reflected across the plane through `origin` at right angles to `normal`. */
export function reflectPoint(p: THREE.Vector3, origin: THREE.Vector3, normal: THREE.Vector3) {
  const away = p.clone().sub(origin).dot(normal);
  return p.clone().addScaledVector(normal, -2 * away);
}

/** The yaw of `forward` turned by `turn`, flat, radians about Y. */
function yawOf(turn: THREE.Quaternion, forward: THREE.Vector3): number {
  const f = forward.clone().applyQuaternion(turn);
  return Math.atan2(f.x, f.z);
}

/**
 * Plays `timeline` on the model of `poser`: `poseFrame` poses bake frame
 * `frame` and answers where round 1's changes stand there (its share).
 */
export function joinComposer(poser: JoinPoser, timeline: Timeline) {
  const { bones, hips } = poser;
  poser.rest();
  const restWorld = new Map(bones.map(b => [b, worldRotation(b)]));
  const hipsRest = hips ? worldRotation(hips) : new THREE.Quaternion();
  const left = new THREE.Vector3().crossVectors(UP, poser.forward).normalize();

  const read = (): Pose => ({
    turns: bones.map(b => b.quaternion.clone()),
    hips: hips ? hips.position.clone() : null,
  });
  const apply = (pose: Pose) => {
    bones.forEach((b, i) => {
      const turn = pose.turns[i];
      if (turn) b.quaternion.copy(turn);
    });
    if (hips && pose.hips) hips.position.copy(pose.hips);
    bones[0]?.parent?.updateWorldMatrix(true, true);
  };
  const lineUpBy = (l: LineUp) => {
    if (!hips || l === NONE) return;
    hips.updateWorldMatrix(true, false);
    const turn = new THREE.Quaternion().setFromAxisAngle(UP, l.yaw);
    const p = worldPosition(hips).sub(l.pivot).applyQuaternion(turn).add(l.pivot).add(l.move);
    const q = turn.multiply(worldRotation(hips));
    setWorldPosition(hips, p);
    setWorldRotation(hips, q);
  };
  const lineUps: LineUp[] = timeline.clips.map(() => NONE);
  const sample = (s: ClipSample): Pose => {
    poser.rest();
    poser.put(s.clip, s.at);
    let pose = read();
    if (s.hold && s.hold.weight > 0) {
      poser.rest();
      poser.put(s.clip, s.hold.at);
      pose = mix(pose, read(), s.hold.weight);
    }
    apply(pose);
    lineUpBy(lineUps[s.clip] ?? NONE);
    return read();
  };
  // Each seam lines its clip up with the clip before as that one is lined up, so the moves add up.
  for (const seam of timeline.seams) {
    if (seam.lineUp === 'off' || !hips) continue;
    apply(sample(seam.from));
    const pA = worldPosition(hips);
    const yawA = yawOf(worldRotation(hips).multiply(hipsRest.clone().invert()), poser.forward);
    apply(sample(seam.to));
    const pB = worldPosition(hips);
    const yawB = yawOf(worldRotation(hips).multiply(hipsRest.clone().invert()), poser.forward);
    lineUps[seam.clip] = {
      yaw: seam.lineUp === 'hips-and-facing' ? yawA - yawB : 0,
      pivot: new THREE.Vector3(pB.x, 0, pB.z),
      move: new THREE.Vector3(pA.x - pB.x, 0, pA.z - pB.z),
    };
  }

  const mirror = () => {
    const world = new Map(bones.map(b => [b, worldRotation(b)]));
    const hipsAt = hips ? worldPosition(hips) : null;
    for (const b of bones) {
      const p = poser.partner(b);
      const delta = (world.get(p) ?? new THREE.Quaternion()).multiply(
        (restWorld.get(p) ?? new THREE.Quaternion()).clone().invert()
      );
      const turned = reflectRotation(delta, left).multiply(
        restWorld.get(b) ?? new THREE.Quaternion()
      );
      setWorldRotation(b, turned);
    }
    if (hips && hipsAt) setWorldPosition(hips, reflectPoint(hipsAt, poser.origin, left));
  };
  const breathe = (frame: number, amount: number) => {
    if (amount <= 0) return;
    const angle =
      BREATHE_ANGLE * amount * Math.sin((2 * Math.PI * frame) / (timeline.fps * BREATHE_SECONDS));
    for (const [bone, share] of [
      [poser.spine, 0.5],
      [poser.chest, 1],
    ] as const) {
      if (!bone) continue;
      const pitch = new THREE.Quaternion().setFromAxisAngle(left, angle * share);
      setWorldRotation(bone, pitch.multiply(worldRotation(bone)));
    }
  };

  return {
    poseFrame: (frame: number): number => {
      const at = Math.min(Math.max(Math.round(frame), 0), timeline.frames);
      const key = timeline.keys[at];
      if (!key) return 0;
      const from = sample(key.from);
      apply(key.to && key.blend > 0 ? mix(from, sample(key.to), key.blend) : from);
      if (timeline.mirror) mirror();
      const held = (s: ClipSample | undefined, w: number) =>
        s?.hold ? s.hold.breathe * s.hold.weight * w : 0;
      breathe(at, held(key.from, 1 - key.blend) + held(key.to, key.blend));
      return key.share;
    },
  };
}

/** `a` moved toward `b` by `w`: each turn slerped, the hips' place lerped. */
function mix(a: Pose, b: Pose, w: number): Pose {
  return {
    turns: a.turns.map((q, i) => q.clone().slerp(b.turns[i] ?? q, w)),
    hips: a.hips && b.hips ? a.hips.clone().lerp(b.hips, w) : a.hips,
  };
}
