/**
 * A 3D preview of one GLB or glTF file: shaded, wireframe, a UV checker or
 * the skeleton, with orbit and zoom. The Inbox uses it today and the
 * Inspector will reuse it. Loaded lazily, so three.js stays out of the
 * first bundle.
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { MeshoptDecoder } from 'three/examples/jsm/libs/meshopt_decoder.module.js';

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

/** An 8 by 8 checker with a number per tile, so stretched UVs show. */
function checkerTexture(): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');
  if (ctx) {
    const tile = size / 8;
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        ctx.fillStyle = (x + y) % 2 === 0 ? '#d9cbb8' : '#5d564d';
        ctx.fillRect(x * tile, y * tile, tile, tile);
        ctx.fillStyle = (x + y) % 2 === 0 ? '#5d564d' : '#d9cbb8';
        ctx.font = '600 20px sans-serif';
        ctx.fillText(`${String.fromCharCode(65 + x)}${y + 1}`, x * tile + 8, y * tile + 26);
      }
    }
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

interface Loaded {
  scene: THREE.Group;
  originals: Map<THREE.Mesh, THREE.Material | THREE.Material[]>;
  skeleton: THREE.SkeletonHelper | null;
}

function useModel(url: string | null, onError?: (message: string) => void): Loaded | null {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  useEffect(() => {
    if (!url) {
      setLoaded(null);
      return;
    }
    let alive = true;
    const loader = new GLTFLoader();
    loader.setMeshoptDecoder(MeshoptDecoder);
    loader.load(
      url,
      gltf => {
        if (!alive) return;
        const scene = gltf.scene;
        const originals = new Map<THREE.Mesh, THREE.Material | THREE.Material[]>();
        let hasBones = false;
        scene.traverse(obj => {
          if ((obj as THREE.Mesh).isMesh) {
            const mesh = obj as THREE.Mesh;
            originals.set(mesh, mesh.material);
            mesh.castShadow = true;
          }
          if ((obj as THREE.Bone).isBone) hasBones = true;
        });
        const skeleton = hasBones ? new THREE.SkeletonHelper(scene) : null;
        setLoaded({ scene, originals, skeleton });
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
  }, [url]);
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

function Fit({
  object,
  onFit,
}: {
  object: THREE.Object3D;
  onFit: (target: THREE.Vector3) => void;
}) {
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  useEffect(() => {
    const box = new THREE.Box3().setFromObject(object);
    if (box.isEmpty()) return;
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    // Stand the model on the floor and centre it.
    object.position.sub(new THREE.Vector3(center.x, box.min.y, center.z));
    const radius = Math.max(size.x, size.y, size.z) * 0.5 || 1;
    const distance = (radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2))) * 1.05;
    camera.position.set(distance * 0.45, size.y * 0.55 + radius * 0.25, distance * 0.9);
    camera.near = distance / 100;
    camera.far = distance * 100;
    camera.lookAt(0, size.y * 0.45, 0);
    camera.updateProjectionMatrix();
    onFit(new THREE.Vector3(0, size.y * 0.45, 0));
  }, [object, camera, onFit]);
  return null;
}

function Model({
  loaded,
  mode,
  onFit,
}: {
  loaded: Loaded;
  mode: PreviewMode;
  onFit: (target: THREE.Vector3) => void;
}) {
  const checker = useMemo(() => checkerTexture(), []);
  useEffect(() => {
    const swapped: THREE.Material[] = [];
    for (const [mesh, original] of loaded.originals) {
      const list = Array.isArray(original) ? original : [original];
      if (mode === 'shaded') {
        mesh.material = original;
        continue;
      }
      const next = list.map(m => {
        if (mode === 'uv') {
          return new THREE.MeshStandardMaterial({ map: checker, roughness: 0.9 });
        }
        const copy = m.clone() as THREE.MeshStandardMaterial;
        if (mode === 'wireframe') copy.wireframe = true;
        if (mode === 'bones') {
          copy.transparent = true;
          copy.opacity = 0.35;
          copy.depthWrite = false;
        }
        return copy;
      });
      swapped.push(...next);
      mesh.material = Array.isArray(original) ? next : (next[0] ?? original);
    }
    return () => swapped.forEach(m => m.dispose());
  }, [loaded, mode, checker]);
  return (
    <>
      <primitive object={loaded.scene} />
      {mode === 'bones' && loaded.skeleton ? <primitive object={loaded.skeleton} /> : null}
      <Fit object={loaded.scene} onFit={onFit} />
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
  const loaded = useModel(url, onError);
  const [target, setTarget] = useState<THREE.Vector3 | null>(null);
  const reported = useRef<Loaded | null>(null);
  useEffect(() => {
    if (!loaded || reported.current === loaded) return;
    reported.current = loaded;
    let bones = 0;
    loaded.scene.traverse(o => {
      if ((o as THREE.Bone).isBone) bones += 1;
    });
    onLoaded?.({ meshes: loaded.originals.size, bones });
  }, [loaded, onLoaded]);
  return (
    <Canvas
      className={className}
      camera={{ fov: 32, position: [2, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      aria-label="Model preview"
    >
      <color attach="background" args={['#7f8083']} />
      <hemisphereLight args={['#f4efe6', '#4a463f', 1.4]} />
      <directionalLight position={[3, 5, 4]} intensity={2.2} />
      <directionalLight position={[-4, 2, -3]} intensity={0.6} />
      <gridHelper args={[10, 20, '#6c6d70', '#76777a']} />
      {loaded ? <Model loaded={loaded} mode={mode} onFit={setTarget} /> : null}
      <Controls turntable={turntable} target={target} />
    </Canvas>
  );
}
