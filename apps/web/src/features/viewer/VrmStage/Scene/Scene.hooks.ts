import { useEffect, useEffectEvent, useMemo, useRef, useState, type RefObject } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import type { GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMUtils, type VRM } from '@pixiv/three-vrm';
import { BACKDROP, castShadows, frameCamera, standOnFloor, type Framing } from '../../studio';
import { poseHumanoid } from '../../vrm-pose';
import { summarize, vrmLoader, vrmOf } from '../vrm-load';
import type { SceneProps } from './Scene.types';

export interface LoadedVrm {
  vrm: VRM;
  framing: Framing;
}

/**
 * Loads the `.vrm` at `url`, again whenever it changes, stands it on the
 * floor, frames the camera on it and tells `onLoaded` what it holds; null
 * while there is none. A file that is plain glTF, with no VRM in it, is an
 * error. The VRM it replaces is freed.
 */
export function useVrm(
  url: string,
  onLoaded: SceneProps['onLoaded'],
  onError: SceneProps['onError']
): LoadedVrm | null {
  const gl = useThree(s => s.gl);
  const scene = useThree(s => s.scene);
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const [loaded, setLoaded] = useState<LoadedVrm | null>(null);
  const fail = useEffectEvent((message: string) => onError?.(message));
  const report = useEffectEvent((gltf: GLTF, vrm: VRM) => onLoaded?.(summarize(gltf, vrm)));
  useEffect(() => {
    scene.background = new THREE.Color(BACKDROP);
  }, [scene]);
  useEffect(() => {
    setLoaded(null);
    let alive = true;
    vrmLoader(gl).load(
      url,
      gltf => {
        if (!alive) return;
        const vrm = vrmOf(gltf);
        if (!vrm) {
          fail('This file is glTF with no VRM in it.');
          return;
        }
        const framing = standOnFloor(vrm.scene);
        if (!framing) {
          fail('This VRM has nothing to draw.');
          return;
        }
        // The springs start from where the model stands now.
        vrm.springBoneManager?.setInitState();
        castShadows(vrm.scene);
        frameCamera(camera, framing);
        setLoaded({ vrm, framing });
        report(gltf, vrm);
      },
      undefined,
      err => {
        if (alive) fail(err instanceof Error ? err.message : 'The VRM did not load.');
      }
    );
    return () => {
      alive = false;
    };
  }, [url, gl, camera]);
  // A VRM replaced by the next file, or left on unmount, frees its GPU memory.
  useEffect(
    () => () => {
      if (loaded) VRMUtils.deepDispose(loaded.vrm.scene);
    },
    [loaded]
  );
  return loaded;
}

/**
 * Where the pointer is over the canvas, as normalised device coordinates;
 * the middle until it first moves, and where it last was once it leaves.
 */
export function usePointer(): RefObject<THREE.Vector2> {
  const gl = useThree(s => s.gl);
  const pointer = useRef(new THREE.Vector2());
  useEffect(() => {
    const element = gl.domElement;
    const move = (event: PointerEvent) => {
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) return;
      pointer.current.set(
        ((event.clientX - box.left) / box.width) * 2 - 1,
        -((event.clientY - box.top) / box.height) * 2 + 1
      );
    };
    element.addEventListener('pointermove', move);
    return () => element.removeEventListener('pointermove', move);
  }, [gl]);
  return pointer;
}

/**
 * Poses the VRM every frame (arms down, breathing), points its look at the
 * pointer, and runs `vrm.update` so the look at, expressions and spring
 * bones move. With the look at off the eyes go back to rest.
 */
export function useIdle(
  vrm: VRM | null,
  { armsDown, breathe, lookAt }: Pick<SceneProps, 'armsDown' | 'breathe' | 'lookAt'>
): THREE.Object3D {
  const camera = useThree(s => s.camera);
  const pointer = usePointer();
  const target = useMemo(() => new THREE.Object3D(), []);
  const clock = useRef(0);
  useEffect(() => {
    if (!vrm?.lookAt) return;
    vrm.lookAt.target = lookAt ? target : null;
    if (!lookAt) vrm.lookAt.reset();
  }, [vrm, lookAt, target]);
  useFrame((_, delta) => {
    if (!vrm) return;
    clock.current += delta;
    poseHumanoid(vrm.humanoid, clock.current, { armsDown, breathe });
    if (lookAt) target.position.set(pointer.current.x, pointer.current.y, 0.5).unproject(camera);
    vrm.update(delta);
  });
  return target;
}
