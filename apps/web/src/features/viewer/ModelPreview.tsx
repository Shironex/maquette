/**
 * A 3D preview of one GLB or glTF file in the studio (see `studio.ts`):
 * shaded, a wireframe over flat clay, a UV checker, or the skeleton drawn in
 * clay over a ghost of the mesh, with orbit and zoom. Reads meshopt geometry
 * and KTX2 textures. The Inbox uses it; the Inspector shares its studio and
 * materials for the compare view.
 * Loaded lazily, so three.js stays out of the first bundle.
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import {
  BACKDROP,
  CLAY,
  castShadows,
  frameCamera,
  gltfLoader,
  standOnFloor,
  studioRig,
  type Framing,
} from './studio';
import { applyMode, checkerTexture, originalsOf, type Originals } from './materials';

export type PreviewMode = 'shaded' | 'wireframe' | 'uv' | 'bones';

export interface ModelPreviewProps {
  /** A URL the web view can fetch: the asset protocol, or a mock file. */
  url: string | null;
  mode?: PreviewMode;
  /** Spin slowly until someone drags. */
  turntable?: boolean;
  onLoaded?: (info: { meshes: number; bones: number }) => void;
  onError?: (message: string) => void;
  className?: string;
}

interface Loaded {
  scene: THREE.Group;
  originals: Originals;
  bones: THREE.Bone[];
}

function useModel(url: string | null, onError?: (message: string) => void): Loaded | null {
  const gl = useThree(s => s.gl);
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  useEffect(() => {
    if (!url) {
      setLoaded(null);
      return;
    }
    let alive = true;
    gltfLoader(gl).load(
      url,
      gltf => {
        if (!alive) return;
        const scene = gltf.scene;
        const originals = originalsOf(scene);
        const bones: THREE.Bone[] = [];
        scene.traverse(obj => {
          if ((obj as THREE.Bone).isBone) bones.push(obj as THREE.Bone);
        });
        castShadows(scene);
        setLoaded({ scene, originals, bones });
      },
      undefined,
      err => {
        if (alive) onError?.(err instanceof Error ? err.message : 'The model did not load.');
      }
    );
    return () => {
      alive = false;
    };
    // onError is a callback prop; reloading on its identity would refetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [url, gl]);
  return loaded;
}

function Controls({ turntable, target }: { turntable: boolean; target: THREE.Vector3 | null }) {
  const { camera, gl } = useThree();
  const controls = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  useEffect(() => {
    if (target) controls.target.copy(target);
  }, [controls, target]);
  useEffect(() => {
    controls.enableDamping = true;
    controls.autoRotate = turntable;
    controls.autoRotateSpeed = 0.8;
    const stop = () => {
      controls.autoRotate = false;
    };
    controls.addEventListener('start', stop);
    return () => {
      controls.removeEventListener('start', stop);
      controls.dispose();
    };
  }, [controls, turntable]);
  useFrame(() => controls.update());
  return null;
}

function Model({ loaded, mode }: { loaded: Loaded; mode: PreviewMode }) {
  const checker = useMemo(() => checkerTexture(), []);
  useEffect(() => applyMode(loaded.originals, mode, checker).undo, [loaded, mode, checker]);
  return <primitive object={loaded.scene} />;
}

/**
 * The skeleton in clay: a tapered strut from each bone to its parent and a
 * ball per joint, drawn over the ghosted mesh. Sized to the model so it reads
 * at any scale, and re-posed every frame so it follows animation.
 */
function Skeleton({ bones, extent }: { bones: THREE.Bone[]; extent: number }) {
  const { struts, joints, pairs } = useMemo(() => {
    const pairs = bones.filter(b => (b.parent as THREE.Bone | null)?.isBone);
    const material = new THREE.MeshBasicMaterial({
      color: CLAY,
      depthTest: false,
      transparent: true,
      opacity: 0.95,
    });
    const jointMaterial = new THREE.MeshBasicMaterial({
      color: '#f3c9b3',
      depthTest: false,
      transparent: true,
    });
    // A unit strut along +Y from 0 to 1, wide at the parent end.
    const strut = new THREE.CylinderGeometry(0.35, 1, 1, 6, 1);
    strut.translate(0, 0.5, 0);
    const struts = new THREE.InstancedMesh(strut, material, Math.max(pairs.length, 1));
    const joints = new THREE.InstancedMesh(
      new THREE.SphereGeometry(1, 10, 8),
      jointMaterial,
      Math.max(bones.length, 1)
    );
    struts.count = pairs.length;
    joints.count = bones.length;
    struts.renderOrder = 10;
    joints.renderOrder = 11;
    struts.frustumCulled = false;
    joints.frustumCulled = false;
    return { struts, joints, pairs };
  }, [bones]);
  useEffect(
    () => () => {
      for (const o of [struts, joints]) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
        o.dispose();
      }
    },
    [struts, joints]
  );
  const scratch = useMemo(
    () => ({
      from: new THREE.Vector3(),
      to: new THREE.Vector3(),
      dir: new THREE.Vector3(),
      up: new THREE.Vector3(0, 1, 0),
      turn: new THREE.Quaternion(),
      size: new THREE.Vector3(),
      matrix: new THREE.Matrix4(),
    }),
    []
  );
  useFrame(() => {
    const { from, to, dir, up, turn, size, matrix } = scratch;
    const width = extent * 0.006;
    pairs.forEach((bone, i) => {
      (bone.parent as THREE.Object3D).getWorldPosition(from);
      bone.getWorldPosition(to);
      dir.subVectors(to, from);
      const length = dir.length();
      if (length > 1e-6) turn.setFromUnitVectors(up, dir.divideScalar(length));
      size.set(width, length, width);
      matrix.compose(from, turn, size);
      struts.setMatrixAt(i, matrix);
    });
    struts.instanceMatrix.needsUpdate = true;
    const r = extent * 0.007;
    bones.forEach((bone, i) => {
      bone.getWorldPosition(from);
      matrix.compose(from, turn.identity(), size.set(r, r, r));
      joints.setMatrixAt(i, matrix);
    });
    joints.instanceMatrix.needsUpdate = true;
  });
  return (
    <>
      <primitive object={struts} />
      <primitive object={joints} />
    </>
  );
}

function Studio({
  url,
  mode,
  turntable,
  onLoaded,
  onError,
}: Omit<ModelPreviewProps, 'className'> & { mode: PreviewMode; turntable: boolean }) {
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const loaded = useModel(url, onError);
  const [framing, setFraming] = useState<Framing | null>(null);
  const reported = useRef<Loaded | null>(null);
  useEffect(() => {
    if (!loaded) return;
    const next = standOnFloor(loaded.scene);
    if (!next) return;
    frameCamera(camera, next);
    setFraming(next);
    if (reported.current !== loaded) {
      reported.current = loaded;
      onLoaded?.({ meshes: loaded.originals.size, bones: loaded.bones.length });
    }
    // onLoaded is a callback prop; reframing on its identity would jump the camera.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded, camera]);
  const rig = useMemo(() => (framing ? studioRig(framing) : null), [framing]);
  useEffect(
    () => () => {
      rig?.traverse(o => {
        const mesh = o as THREE.Mesh;
        if (!mesh.isMesh) return;
        mesh.geometry.dispose();
        const material = mesh.material as THREE.MeshBasicMaterial;
        material.map?.dispose();
        material.dispose();
      });
    },
    [rig]
  );
  return (
    <>
      {rig ? <primitive object={rig} /> : null}
      {loaded ? <Model loaded={loaded} mode={mode} /> : null}
      {loaded && framing && mode === 'bones' && loaded.bones.length > 0 ? (
        <Skeleton bones={loaded.bones} extent={framing.extent} />
      ) : null}
      <Controls turntable={turntable} target={framing?.target ?? null} />
    </>
  );
}

export default function ModelPreview({
  url,
  mode = 'shaded',
  turntable = true,
  onLoaded,
  onError,
  className,
}: ModelPreviewProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [2, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      aria-label="Model preview"
    >
      <color attach="background" args={[BACKDROP]} />
      <Studio url={url} mode={mode} turntable={turntable} onLoaded={onLoaded} onError={onError} />
    </Canvas>
  );
}
