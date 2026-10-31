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
import { disposeObject } from '@/features/viewer/dispose';
import type { EditorProps, EditorRig, Flight } from './Editor.types';
import { useClickSelect } from './use-click-select';
import { useEditorCamera } from './use-editor-camera';
import { useEditorGizmo } from './use-editor-gizmo';
import { useEditorScene } from './use-editor-scene';
import { LOAD_AT_ONCE, inTurns, uploadTextures } from './load-in-turns';

/**
 * Loads every model once, a few at a time with turns for the window between,
 * so the tabs stay clickable; each sits in a holder the layout moves. Leaving
 * starts nothing more and frees what still arrives. `onProgress` hears how
 * many are done.
 */
export function useHolders(
  models: LayoutModel[],
  onError: (message: string) => void,
  onProgress?: (done: number) => void
): Map<string, THREE.Group> | null {
  const gl = useThree(s => s.gl);
  const [holders, setHolders] = useState<Map<string, THREE.Group> | null>(null);
  // Only a different set of files loads again, not a new array of the same.
  const key = models.map(m => `${m.asset}|${m.file}`).join('\n');
  const current = useEffectEvent(() => models);
  const fail = useEffectEvent(onError);
  const progress = useEffectEvent((done: number) => onProgress?.(done));
  useEffect(() => {
    let live = true;
    const alive = () => live;
    const made = new Map<string, THREE.Group>();
    const loader = gltfLoader(gl);
    const list = current();
    let done = 0;
    void inTurns(list, LOAD_AT_ONCE, alive, async model => {
      try {
        const url = fileUrl(model.file);
        if (!url) throw new Error(`${model.name} has no file to show.`);
        const gltf = await loader.loadAsync(url);
        if (!live) {
          disposeObject(gltf.scene);
          return;
        }
        const holder = new THREE.Group();
        holder.name = model.asset;
        holder.userData.asset = model.asset;
        holder.add(gltf.scene);
        castShadows(holder);
        holder.traverse(o => {
          if ((o as THREE.Mesh).isMesh) o.receiveShadow = true;
        });
        made.set(model.asset, holder);
        await uploadTextures(gl, holder, alive);
      } finally {
        done += 1;
        if (live) progress(done);
      }
    }).then(failed => {
      if (!live) return;
      if (failed > 0) {
        fail(
          failed === list.length
            ? 'The models did not load.'
            : `${failed} of ${list.length} models did not load.`
        );
      }
      setHolders(made);
    });
    return () => {
      live = false;
      for (const holder of made.values()) disposeObject(holder);
    };
  }, [key, gl]);
  return holders;
}

/** The renderer, camera, controls, groups and refs the editor's hooks share. */
export function useEditorRig(props: EditorProps): EditorRig {
  const { gl, scene, size } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const holders = useHolders(props.models, props.handlers.onError, props.handlers.onProgress);
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
