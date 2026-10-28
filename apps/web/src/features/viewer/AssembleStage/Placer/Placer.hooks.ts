import { useEffect, useEffectEvent, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import { fileUrl } from '@/ipc/files';
import { BACKDROP, castShadows, frameCamera, gltfLoader, studioRig } from '../../studio';
import type { AssembleStageProps, StagePart } from '../AssembleStage.types';
import { pivotPoint, placeHolder, placeOf } from './place';

type PlacerProps = Omit<AssembleStageProps, 'className'>;

/** Frees the geometry, materials and textures under `root`. */
function dispose(root: THREE.Object3D): void {
  root.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.geometry.dispose();
    for (const material of [mesh.material].flat()) {
      for (const value of Object.values(material)) {
        if (value instanceof THREE.Texture) value.dispose();
      }
      material.dispose();
    }
  });
}

/** Loads every part's model once; each sits in a holder named for its asset. */
function useHolders(
  parts: StagePart[],
  gl: THREE.WebGLRenderer,
  onError: (message: string) => void
): Map<string, THREE.Group> | null {
  const [holders, setHolders] = useState<Map<string, THREE.Group> | null>(null);
  // Only a different set of files loads again, not a new place for a part.
  const key = parts.map(p => `${p.asset}|${p.file}`).join('\n');
  const current = useEffectEvent(() => parts);
  const fail = useEffectEvent(onError);
  useEffect(() => {
    let alive = true;
    const made = new Map<string, THREE.Group>();
    const loader = gltfLoader(gl);
    const list = current();
    Promise.allSettled(
      list.map(async part => {
        const url = fileUrl(part.file);
        if (!url) throw new Error(`${part.asset} has no file to show.`);
        const gltf = await loader.loadAsync(url);
        const holder = new THREE.Group();
        holder.userData.asset = part.asset;
        holder.add(gltf.scene);
        castShadows(holder);
        made.set(part.asset, holder);
      })
    ).then(results => {
      if (!alive) return;
      const failed = results.filter(r => r.status === 'rejected').length;
      if (failed > 0) fail(`${failed} of ${results.length} parts did not load.`);
      setHolders(made);
    });
    return () => {
      alive = false;
      for (const holder of made.values()) dispose(holder);
    };
  }, [key, gl]);
  return holders;
}

/** The studio's lights and floor, and the camera, framed to every part once they load. */
function useFraming(
  holders: Map<string, THREE.Group> | null,
  world: THREE.Group,
  orbit: OrbitControls
): void {
  const { scene, camera } = useThree();
  useEffect(() => {
    if (!holders || holders.size === 0) return;
    world.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(world);
    if (box.isEmpty()) return;
    const framing = {
      box,
      extent: box.getSize(new THREE.Vector3()).length() || 1,
      target: box.getCenter(new THREE.Vector3()),
    };
    const rig = studioRig(framing);
    scene.add(rig);
    frameCamera(camera as THREE.PerspectiveCamera, framing);
    orbit.target.copy(framing.target);
    return () => {
      scene.remove(rig);
      dispose(rig);
    };
  }, [holders, world, orbit, scene, camera]);
}

/** A click (not a drag, not on the gizmo) selects the part under the pointer. */
function useClickSelect(
  world: THREE.Group,
  transform: TransformControls,
  live: { current: PlacerProps }
) {
  const { gl, camera } = useThree();
  useEffect(() => {
    const el = gl.domElement;
    const raycaster = new THREE.Raycaster();
    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = { x: e.clientX, y: e.clientY };
    };
    const onUp = (e: PointerEvent) => {
      if (!down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
      down = null;
      if (transform.dragging || transform.axis) return;
      const rect = el.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      for (const hit of raycaster.intersectObject(world, true)) {
        let node: THREE.Object3D | null = hit.object;
        while (node && node.userData.asset === undefined) node = node.parent;
        if (node) return live.current.onSelect(node.userData.asset as string);
      }
      live.current.onSelect(null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, world, transform, live]);
}

/**
 * The assembly: each part's holder where its place and turn put it (a part
 * with no place yet reports where its model put it), the gizmo on the
 * selected part, its drags reported as places and turns, and click to select.
 */
export function usePlacer(props: PlacerProps): void {
  const { gl, scene } = useThree();
  const camera = useThree(s => s.camera);
  const live = useRef(props);
  useLayoutEffect(() => {
    live.current = props;
  });
  const orbit = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  const transform = useMemo(() => new TransformControls(camera, gl.domElement), [camera, gl]);
  const world = useMemo(() => new THREE.Group(), []);
  const holders = useHolders(props.parts, gl, props.onError);

  useEffect(() => {
    scene.background = new THREE.Color(BACKDROP);
    scene.add(world, transform.getHelper());
    return () => {
      scene.remove(world, transform.getHelper());
    };
  }, [scene, world, transform]);

  useEffect(() => {
    world.clear();
    if (holders) world.add(...holders.values());
  }, [world, holders]);

  useEffect(() => {
    for (const part of props.parts) {
      const holder = holders?.get(part.asset);
      const model = holder?.children[0];
      if (!holder || !model) continue;
      placeHolder(holder, pivotPoint(model, part.pivot), part.at, part.turn);
      if (!part.at) live.current.onPlace(part.asset, placeOf(holder).at, part.turn);
    }
  }, [holders, props.parts]);
  useFraming(holders, world, orbit);

  useEffect(() => {
    const holder = props.selected ? holders?.get(props.selected) : undefined;
    if (holder) transform.attach(holder);
    else transform.detach();
  }, [holders, props.selected, transform]);

  const { mode, snap } = props;
  useEffect(() => {
    const move = mode === 'translate';
    transform.setMode(mode);
    transform.setSpace('world');
    // A turn is only about the up axis, so the other rings stay hidden.
    transform.showX = move;
    transform.showZ = move;
    transform.setTranslationSnap(snap.move);
    transform.setRotationSnap(snap.turn === null ? null : THREE.MathUtils.degToRad(snap.turn));
    transform.setSize(0.8);
  }, [transform, mode, snap.move, snap.turn]);

  useEffect(() => {
    orbit.enableDamping = true;
    orbit.dampingFactor = 0.1;
    const onDragging = (e: { value: unknown }) => {
      orbit.enabled = !e.value;
    };
    const onChange = () => {
      const object = transform.object;
      const asset = object?.userData.asset as string | undefined;
      if (!object || !asset) return;
      const { at, turn } = placeOf(object);
      live.current.onPlace(asset, at, turn);
    };
    transform.addEventListener('dragging-changed', onDragging);
    transform.addEventListener('objectChange', onChange);
    return () => {
      transform.removeEventListener('dragging-changed', onDragging);
      transform.removeEventListener('objectChange', onChange);
      transform.detach();
      transform.dispose();
      orbit.dispose();
    };
  }, [transform, orbit]);

  useClickSelect(world, transform, live);
  useFrame(() => orbit.update());
}
