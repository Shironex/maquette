/**
 * Two versions of a scene or an asset in one canvas, split down the middle:
 * each side is its own three.js scene, both drawn with the same camera, the
 * left one scissored to the left of the split and the right one to the
 * rest, so orbiting moves both at once. Reuses the preview studio (lights,
 * floor, framing) and its material modes.
 *
 * It also measures itself: every fourth frame, and the first of each
 * sample, waits for the GPU to finish,
 * so its time covers updating and drawing both sides on the CPU and the
 * GPU; the samples average over 250 ms, next to the frame rate.
 * Loaded lazily, with three.js.
 */
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { clone as cloneSkinned } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { FLIGHT_MS, SAMPLE_EVERY_MS, flight, type CameraPose, type Stop } from '@/state/inspector';
import type { InspectMode } from '@/state/inspector-store';
import {
  applyMode,
  checkerTexture,
  originalsOf,
  type Originals,
} from '@/features/viewer/materials';
import {
  BACKDROP,
  castShadows,
  frameCamera,
  gltfLoader,
  standOnFloor,
  studioRig,
  type Framing,
} from '@/features/viewer/studio';

export interface CompareModel {
  /** The InspectModel id, reported back when it is clicked. */
  id: string;
  url: string;
  position: [number, number, number];
}

export interface CompareCanvasProps {
  kind: 'scene' | 'asset';
  left: CompareModel[];
  right: CompareModel[];
  /** Where the split sits, from 0 (all right side) to 1 (all left side). */
  split: number;
  mode: InspectMode;
  turntable: boolean;
  stops: Stop[];
  stop: { index: number; at: number } | null;
  onSample: (ms: number, fps: number) => void;
  onPick: (model: string | null) => void;
  /** Live renders of the stops without a poster, by stop index. */
  onStopShots: (shots: Record<number, string>) => void;
  /** The camera was moved by hand. */
  onUserMove: () => void;
  onLightmaps: (meshes: number) => void;
  onError: (message: string) => void;
  onReady: () => void;
  className?: string;
}

interface Side {
  group: THREE.Group;
  originals: Originals;
  mixers: THREE.AnimationMixer[];
}

const SHOT = { width: 320, height: 180 };
/** Frames between two timed (GPU synced) frames. */
const TIME_EVERY = 4;

function disposeGroup(group: THREE.Object3D) {
  group.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.geometry.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    materials.forEach(m => m.dispose());
  });
}

/** Loads every model of one side, each file once, placed at its position. */
function useSide(models: CompareModel[], onError: (message: string) => void): Side | null {
  const gl = useThree(s => s.gl);
  const [side, setSide] = useState<Side | null>(null);
  const key = JSON.stringify(models);
  useEffect(() => {
    if (models.length === 0) {
      setSide(null);
      return;
    }
    let alive = true;
    let made: THREE.Group | null = null;
    const loader = gltfLoader(gl);
    const urls = [...new Set(models.map(m => m.url))];
    Promise.all(urls.map(url => loader.loadAsync(url)))
      .then(loaded => {
        if (!alive) return;
        const byUrl = new Map(urls.map((url, i) => [url, loaded[i]!]));
        const used = new Set<string>();
        const group = new THREE.Group();
        const mixers: THREE.AnimationMixer[] = [];
        for (const model of models) {
          const gltf = byUrl.get(model.url)!;
          // A file placed twice needs its own copy, skeleton included.
          const scene = used.has(model.url)
            ? (cloneSkinned(gltf.scene) as THREE.Group)
            : gltf.scene;
          used.add(model.url);
          const holder = new THREE.Group();
          holder.position.set(...model.position);
          holder.userData.modelId = model.id;
          holder.add(scene);
          group.add(holder);
          const clip = gltf.animations[0];
          if (clip) {
            const mixer = new THREE.AnimationMixer(scene);
            mixer.clipAction(clip).play();
            mixers.push(mixer);
          }
        }
        castShadows(group);
        made = group;
        setSide({ group, originals: originalsOf(group), mixers });
      })
      .catch((err: unknown) => {
        if (alive) onError(err instanceof Error ? err.message : 'A model did not load.');
      });
    return () => {
      alive = false;
      if (made) disposeGroup(made);
    };
    // `key` stands for `models`; onError is a callback prop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, gl]);
  return side;
}

/** The bounds of `group` without moving it, for a scene that keeps its layout. */
function sceneFraming(group: THREE.Object3D): Framing | null {
  group.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(group);
  if (box.isEmpty()) return null;
  const extent = box.getSize(new THREE.Vector3()).length() || 1;
  return { box, extent, target: box.getCenter(new THREE.Vector3()) };
}

function poseOf(camera: THREE.PerspectiveCamera, target: THREE.Vector3): CameraPose {
  return {
    position: camera.position.toArray() as [number, number, number],
    target: target.toArray() as [number, number, number],
    fov: camera.fov,
  };
}

function setPose(camera: THREE.PerspectiveCamera, controls: OrbitControls, pose: CameraPose) {
  camera.position.set(...pose.position);
  controls.target.set(...pose.target);
  if (camera.fov !== pose.fov) {
    camera.fov = pose.fov;
    camera.updateProjectionMatrix();
  }
  camera.lookAt(controls.target);
}

function Compare(props: Omit<CompareCanvasProps, 'className'>) {
  const { gl, size } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const left = useSide(props.left, props.onError);
  const right = useSide(props.right, props.onError);
  const scenes = useMemo(() => {
    const make = () => {
      const scene = new THREE.Scene();
      scene.background = new THREE.Color(BACKDROP);
      return scene;
    };
    return { left: make(), right: make() };
  }, []);
  const controls = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  const checker = useMemo(() => checkerTexture(), []);
  const [framing, setFraming] = useState<Framing | null>(null);
  const live = useRef(props);
  useLayoutEffect(() => {
    live.current = props;
  });
  const flightRef = useRef<{ from: CameraPose; stop: Stop; start: number } | null>(null);
  const shotsPending = useRef(false);
  const stats = useRef({ windowStart: 0, frames: 0, timed: 0, work: 0, last: 0 });
  const pixel = useMemo(() => new Uint8Array(4), []);
  const raycaster = useMemo(() => new THREE.Raycaster(), []);

  // Both sides in place: frame on the current (right) side and light both.
  useEffect(() => {
    const main = right ?? left;
    if (!main) return;
    let next: Framing | null;
    if (props.kind === 'asset') {
      if (left && left !== main) standOnFloor(left.group);
      next = standOnFloor(main.group);
    } else {
      next = sceneFraming(main.group);
    }
    if (!next) return;
    const rigs: THREE.Group[] = [];
    for (const [scene, side] of [
      [scenes.left, left],
      [scenes.right, right],
    ] as const) {
      scene.clear();
      if (!side) continue;
      const rig = studioRig(next);
      if (props.kind === 'scene') rig.position.y = next.box.min.y;
      rigs.push(rig);
      scene.add(rig, side.group);
    }
    camera.near = next.extent / 400;
    camera.far = next.extent * 60;
    camera.updateProjectionMatrix();
    setFraming(next);
    live.current.onReady();
    return () => {
      for (const rig of rigs) {
        rig.removeFromParent();
        disposeGroup(rig);
      }
    };
  }, [left, right, props.kind, scenes, camera]);

  // The first view: the current stop, or the studio's three quarter framing.
  useEffect(() => {
    if (!framing) return;
    const p = live.current;
    const stop = p.kind === 'scene' && p.stop ? p.stops[p.stop.index] : undefined;
    if (stop) {
      setPose(camera, controls, { position: stop.position, target: stop.target, fov: stop.fov });
    } else {
      camera.fov = 30;
      frameCamera(camera, framing);
      controls.target.copy(framing.target);
    }
    shotsPending.current = p.kind === 'scene' && p.stops.some(s => !s.poster);
  }, [framing, camera, controls]);

  // A stop was clicked: fly there.
  useEffect(() => {
    if (!framing || !props.stop || props.kind !== 'scene') return;
    const stop = props.stops[props.stop.index];
    if (!stop) return;
    flightRef.current = { from: poseOf(camera, controls.target), stop, start: performance.now() };
    controls.autoRotate = false;
    // Only a new click starts a flight; the stops list changing does not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.stop?.at, framing]);

  useEffect(() => {
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.autoRotateSpeed = 0.6;
    const start = () => {
      flightRef.current = null;
      controls.autoRotate = false;
      live.current.onUserMove();
    };
    controls.addEventListener('start', start);
    return () => {
      controls.removeEventListener('start', start);
      controls.dispose();
    };
  }, [controls]);

  useEffect(() => {
    controls.autoRotate = props.turntable;
  }, [controls, props.turntable]);

  // Material modes on both sides.
  useEffect(() => {
    const applied = [left, right]
      .filter((s): s is Side => Boolean(s))
      .map(s => applyMode(s.originals, props.mode, checker));
    if (props.mode === 'lightmap') {
      live.current.onLightmaps(applied.reduce((n, a) => n + a.lightmapped, 0));
    }
    return () => applied.forEach(a => a.undo());
  }, [left, right, props.mode, checker]);

  // A click (not a drag) picks the model under the pointer, on its side.
  useEffect(() => {
    const el = gl.domElement;
    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = { x: e.clientX, y: e.clientY };
    };
    const onUp = (e: PointerEvent) => {
      if (!down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
      down = null;
      const rect = el.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const p = live.current;
      const side = x < rect.width * p.split ? left : right;
      if (!side) return;
      const ndc = new THREE.Vector2(
        (x / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const hit = raycaster.intersectObject(side.group, true)[0];
      let node: THREE.Object3D | null = hit?.object ?? null;
      while (node && node.userData.modelId === undefined) node = node.parent;
      p.onPick((node?.userData.modelId as string | undefined) ?? null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, raycaster, left, right]);

  /** Renders every stop without a poster into a small JPEG, inside one frame. */
  function shootStops() {
    const p = live.current;
    const shots: Record<number, string> = {};
    const canvas = document.createElement('canvas');
    canvas.width = SHOT.width;
    canvas.height = SHOT.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const shot = camera.clone();
    const el = gl.domElement;
    shot.aspect = el.width / el.height;
    gl.setScissorTest(false);
    gl.setViewport(0, 0, size.width, size.height);
    p.stops.forEach((stop, i) => {
      if (stop.poster) return;
      shot.fov = stop.fov;
      shot.position.set(...stop.position);
      shot.lookAt(...stop.target);
      shot.updateProjectionMatrix();
      gl.render(scenes.right, shot);
      // Cover crop to 16:9.
      const want = SHOT.width / SHOT.height;
      const sw = Math.min(el.width, el.height * want);
      const sh = sw / want;
      ctx.drawImage(
        el,
        (el.width - sw) / 2,
        (el.height - sh) / 2,
        sw,
        sh,
        0,
        0,
        SHOT.width,
        SHOT.height
      );
      shots[i] = canvas.toDataURL('image/jpeg', 0.84);
    });
    p.onStopShots(shots);
  }

  useFrame((_, delta) => {
    const start = performance.now();
    const p = live.current;
    for (const side of [left, right]) side?.mixers.forEach(m => m.update(delta));
    const f = flightRef.current;
    if (f) {
      const elapsed = start - f.start;
      setPose(camera, controls, flight(f.from, f.stop, elapsed));
      if (elapsed >= FLIGHT_MS) flightRef.current = null;
    } else {
      controls.update();
    }
    let shooting = false;
    if (shotsPending.current && right && framing) {
      shotsPending.current = false;
      shooting = true;
      shootStops();
    }
    const w = size.width;
    const h = size.height;
    const at = left ? Math.round(w * p.split) : 0;
    gl.setViewport(0, 0, w, h);
    gl.setScissorTest(true);
    if (left && at > 0) {
      gl.setScissor(0, 0, at, h);
      gl.render(scenes.left, camera);
    }
    if (at < w) {
      gl.setScissor(at, 0, w - at, h);
      gl.render(scenes.right, camera);
    }
    gl.setScissorTest(false);

    // Every few frames (and at least once per sample), wait for the GPU (a
    // one pixel read) so the sample is the whole cost of the frame, not just
    // the CPU handing work over.
    const s = stats.current;
    s.frames += 1;
    if (!shooting && (s.timed === 0 || s.frames % TIME_EVERY === 0)) {
      const ctx = gl.getContext();
      ctx.readPixels(0, 0, 1, 1, ctx.RGBA, ctx.UNSIGNED_BYTE, pixel);
      s.work += performance.now() - start;
      s.timed += 1;
    }
    const now = performance.now();
    if (s.windowStart === 0) s.windowStart = now;
    const span = now - s.windowStart;
    if (span >= SAMPLE_EVERY_MS) {
      if (s.timed > 0) s.last = s.work / s.timed;
      if (s.last > 0) p.onSample(s.last, (s.frames * 1000) / span);
      s.windowStart = now;
      s.frames = 0;
      s.timed = 0;
      s.work = 0;
    }
  }, 1);

  return null;
}

export default function CompareCanvas({ className, ...props }: CompareCanvasProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [6, 4, 10] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      aria-label="Compare view"
    >
      <Compare {...props} />
    </Canvas>
  );
}
