import { useEffect, useEffectEvent, useLayoutEffect, useMemo, useRef } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import { frameCamera } from '@/features/viewer/studio';
import { useHolders } from '../../LayoutCanvas/Editor/Editor.hooks';
import type { PathsRig, PathsSceneProps } from './PathsScene.types';
import { usePathClicks } from './use-path-clicks';
import { usePathStroke } from './use-path-stroke';
import { usePathTubes } from './use-path-tubes';
import { usePointGizmo } from './use-point-gizmo';

/** The renderer, camera, controls, groups and refs the scene's hooks share. */
export function usePathsRig(props: PathsSceneProps): PathsRig {
  const { gl, scene } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const holders = useHolders(props.models, props.handlers.onError);
  const live = useRef(props);
  useLayoutEffect(() => {
    live.current = props;
  });
  const orbit = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  const transform = useMemo(() => new TransformControls(camera, gl.domElement), [camera, gl]);
  const groups = useMemo(
    () => ({ world: new THREE.Group(), tubes: new THREE.Group(), handles: new THREE.Group() }),
    []
  );
  const raycaster = useMemo(() => new THREE.Raycaster(), []);
  return { gl, scene, camera, holders, live, orbit, transform, ...groups, raycaster };
}

/** Frames the models and the paths: on load, and each time the frame counter moves. */
export function usePathsCamera(rig: PathsRig, frame: number): void {
  const { camera, orbit, holders, world, tubes } = rig;
  useEffect(() => {
    orbit.enableDamping = true;
    orbit.dampingFactor = 0.1;
    orbit.screenSpacePanning = true;
    return () => orbit.dispose();
  }, [orbit]);
  const frameAll = useEffectEvent(() => {
    if (!holders) return;
    world.updateMatrixWorld(true);
    tubes.updateMatrixWorld(true);
    const bounds = new THREE.Box3()
      .setFromObject(world)
      .union(new THREE.Box3().setFromObject(tubes));
    if (bounds.isEmpty()) bounds.set(new THREE.Vector3(-1, 0, -1), new THREE.Vector3(1, 1, 1));
    const target = bounds.getCenter(new THREE.Vector3());
    camera.fov = 35;
    frameCamera(camera, {
      box: bounds,
      extent: bounds.getSize(new THREE.Vector3()).length() || 1,
      target,
    });
    camera.near = 0.01;
    camera.far = 400;
    camera.updateProjectionMatrix();
    orbit.target.copy(target);
  });
  useEffect(() => frameAll(), [holders, frame]);
  useFrame(() => orbit.update());
}

/** The whole scene, in the order its effects must run. */
export function usePathsScene(props: PathsSceneProps): void {
  const rig = usePathsRig(props);
  usePathTubes(rig, props);
  usePointGizmo(rig, props);
  usePathsCamera(rig, props.frame);
  usePathClicks(rig);
  usePathStroke(rig);
}
