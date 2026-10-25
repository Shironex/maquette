import {
  useCallback,
  useEffect,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import type { LayoutModel } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { FLIGHT_MS, flight } from '@/state/inspector';
import { CLAY, castShadows, gltfLoader } from '@/features/viewer/studio';
import { poseOf, setPose } from '../../camera-pose';
import { disposeTree } from './dispose-tree';
import type { EditorProps, EditorRig, Flight } from './Editor.types';
import { useClickSelect } from './use-click-select';
import { useEditorCamera } from './use-editor-camera';
import { useEditorGizmo } from './use-editor-gizmo';
import { useEditorScene } from './use-editor-scene';

/** Loads every model once; each sits in a holder the layout moves. */
export function useHolders(
  models: LayoutModel[],
  onError: (message: string) => void
): Map<string, THREE.Group> | null {
  const gl = useThree(s => s.gl);
  const [holders, setHolders] = useState<Map<string, THREE.Group> | null>(null);
  // Only a different set of files loads again, not a new array of the same.
  const key = models.map(m => `${m.asset}|${m.file}`).join('\n');
  const current = useEffectEvent(() => models);
  const fail = useEffectEvent(onError);
  useEffect(() => {
    let alive = true;
    const made = new Map<string, THREE.Group>();
    const loader = gltfLoader(gl);
    const list = current();
    Promise.allSettled(
      list.map(async model => {
        const url = fileUrl(model.file);
        if (!url) throw new Error(`${model.name} has no file to show.`);
        const gltf = await loader.loadAsync(url);
        const holder = new THREE.Group();
        holder.name = model.asset;
        holder.userData.asset = model.asset;
        holder.add(gltf.scene);
        castShadows(holder);
        holder.traverse(o => {
          if ((o as THREE.Mesh).isMesh) o.receiveShadow = true;
        });
        made.set(model.asset, holder);
      })
    ).then(results => {
      if (!alive) return;
      const failed = results.filter(r => r.status === 'rejected');
      if (failed.length > 0) {
        fail(
          failed.length === results.length
            ? 'The models did not load.'
            : `${failed.length} of ${results.length} models did not load.`
        );
      }
      setHolders(made);
    });
    return () => {
      alive = false;
      for (const holder of made.values()) disposeTree(holder);
    };
  }, [key, gl]);
  return holders;
}

/** The renderer, camera, controls, groups and refs the editor's hooks share. */
export function useEditorRig(props: EditorProps): EditorRig {
  const { gl, scene, size } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const holders = useHolders(props.models, props.handlers.onError);
  const live = useRef(props);
  useLayoutEffect(() => {
    live.current = props;
  });
  const orbit = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  const transform = useMemo(() => new TransformControls(camera, gl.domElement), [camera, gl]);
  const world = useMemo(() => new THREE.Group(), []);
  const stopsGroup = useMemo(() => new THREE.Group(), []);
  const box = useMemo(() => new THREE.Box3Helper(new THREE.Box3(), CLAY), []);
  const raycaster = useMemo(() => new THREE.Raycaster(), []);
  const flightRef = useRef<Flight | null>(null);
  const dragStart = useRef(1);
  const fitted = useRef<{ shown: number; real: number } | null>(null);
  const report = useCallback(() => {
    const pose = poseOf(camera, orbit.target);
    const f = fitted.current;
    if (f && Math.abs(pose.fov - f.shown) < 1e-4) pose.fov = f.real;
    live.current.handlers.onPose(pose);
  }, [camera, orbit]);
  return {
    gl,
    scene,
    size,
    camera,
    holders,
    live,
    orbit,
    transform,
    world,
    stopsGroup,
    box,
    raycaster,
    flightRef,
    dragStart,
    fitted,
    report,
  };
}

/** Each frame: the camera's flight or its orbit, and the selection box on its model. */
export function useEditorFrame(rig: EditorRig): void {
  const { camera, orbit, box, holders, live, flightRef, report } = rig;
  useFrame(() => {
    const f = flightRef.current;
    if (f) {
      const elapsed = performance.now() - f.start;
      setPose(camera, orbit, flight(f.from, { ...f.to, name: '', poster: null }, elapsed));
      if (elapsed >= FLIGHT_MS) {
        flightRef.current = null;
        report();
      }
    } else {
      orbit.update();
    }
    const sel = live.current.selection;
    if (sel?.kind !== 'model' || !box.visible) return;
    const holder = holders?.get(sel.id);
    if (holder) box.box.setFromObject(holder);
  });
}

/**
 * The whole editor, in the order its effects must run: the scene, the
 * gizmo, the camera, click to select and the frame loop.
 */
export function useEditor(props: EditorProps): void {
  const rig = useEditorRig(props);
  const span = useEditorScene(rig, props);
  useEditorGizmo(rig, props, span);
  useEditorCamera(rig, props);
  useClickSelect(rig);
  useEditorFrame(rig);
}
