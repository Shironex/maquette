/**
 * A model whose root travels (a walk, a lunge, joined clips that move on)
 * in a studio framed where it started: the floor's shadow and the lights
 * stay under it, the camera can follow it, and F frames it again where it
 * stands. Orbit and zoom stay the person's: following moves the camera and
 * what it orbits by the same step.
 */
import { useEffect, useEffectEvent, useMemo, useRef } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { typing } from '@/lib/typing';
import { FRAME_KEY } from './frame-key';
import { frameCamera, type Framing } from './studio';

const HIPS = /hips|pelvis/i;

/** The bone the body moves with: the hips (or pelvis), else the topmost bone; null without bones. */
export function moverOf(scene: THREE.Object3D): THREE.Object3D | null {
  const bones: THREE.Object3D[] = [];
  scene.traverse(o => {
    if ((o as THREE.Bone).isBone) bones.push(o);
  });
  const depth = (o: THREE.Object3D) => {
    let n = 0;
    for (let p = o.parent; p && p !== scene; p = p.parent) n += 1;
    return n;
  };
  const hips = bones.find(b => HIPS.test(b.name));
  if (hips) return hips;
  return bones.reduce<THREE.Object3D | null>(
    (best, b) => (!best || depth(b) < depth(best) ? b : best),
    null
  );
}

/** `framing` moved along the floor by `by` (its x and z). */
export function framingAt(framing: Framing, by: THREE.Vector3): Framing {
  const shift = new THREE.Vector3(by.x, 0, by.z);
  return {
    box: framing.box.clone().translate(shift),
    extent: framing.extent,
    target: framing.target.clone().add(shift),
  };
}

export interface RootFollow {
  /** The model's scene, posed each frame. */
  scene: THREE.Object3D | null;
  /** The studio's lights, floor and shadow, moved under the model. */
  rig: THREE.Object3D | null;
  framing: Framing | null;
  controls: OrbitControls | null;
  /** The camera moves with the model. */
  follow: boolean;
  /** F frames the model where it stands. */
  frameKey: boolean;
}

/**
 * Keeps `rig` under the model as its root travels, moves the camera with
 * it while `follow` is on (framing it again when follow turns on), and
 * frames it again on F.
 */
export function useRootFollow({ scene, rig, framing, controls, follow, frameKey }: RootFollow) {
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const mover = useMemo(() => (scene ? moverOf(scene) : null), [scene]);
  const home = useRef<THREE.Vector3 | null>(null);
  const at = useMemo(() => new THREE.Vector3(), []);
  // Where the mover stood when the model was framed, so the studio starts where it was built.
  useEffect(() => {
    home.current = mover && framing ? mover.getWorldPosition(new THREE.Vector3()) : null;
  }, [mover, framing]);
  const frameNow = useEffectEvent(() => {
    if (!framing) return;
    const placed = framingAt(framing, rig?.position ?? new THREE.Vector3());
    frameCamera(camera, placed);
    controls?.target.copy(placed.target);
  });
  useEffect(() => {
    if (follow) frameNow();
  }, [follow]);
  useEffect(() => {
    if (!frameKey) return;
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key.toLowerCase() !== FRAME_KEY.toLowerCase()) return;
      e.preventDefault();
      frameNow();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [frameKey]);
  useFrame(() => {
    if (!mover || !rig || !home.current) return;
    mover.getWorldPosition(at);
    const dx = at.x - home.current.x;
    const dz = at.z - home.current.z;
    const stepX = dx - rig.position.x;
    const stepZ = dz - rig.position.z;
    if (!stepX && !stepZ) return;
    rig.position.set(dx, 0, dz);
    if (!follow) return;
    camera.position.x += stepX;
    camera.position.z += stepZ;
    if (!controls) return;
    controls.target.x += stepX;
    controls.target.z += stepZ;
  });
}
