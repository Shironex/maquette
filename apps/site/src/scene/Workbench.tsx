/**
 * The live workbench: the manifest's models under a warm lamp and cool
 * moonlight, a camera that eases between the tour's stops, the turntable
 * spin and the wireframe sweep. Drawn on demand: frames only render while
 * something moves.
 */
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { buildModel, type BuiltModel } from './build';
import { loadInOrder } from './loader';
import {
  MODELS,
  POSTER_CAMERAS,
  STOPS,
  loadOrder,
  type ModelEntry,
  type PosterCamera,
  type StopId,
} from './manifest';
import { CameraRig } from './CameraRig';
import { Lights } from './Lights';
import { Callouts } from './Callouts';

export interface WorkbenchProps {
  initialStop: StopId;
  reducedMotion: boolean;
  /** Subscribes to stop changes; returns the unsubscribe. */
  subscribe: (listener: (stop: StopId) => void) => () => void;
  onReady: () => void;
  onFailed: (reason: string) => void;
  /** Poster capture only: hold a poster framing and expose the renderer for stats. */
  capture?: PosterCamera | 'stops';
}

/** Spin speed of the turntable, radians per second. */
const SPIN = 0.32;
/** One pass of the wireframe sweep, then a short hold, in seconds. */
const SWEEP_PASS = 2.6;
const SWEEP_HOLD = 1.2;

function useStop(initial: StopId, subscribe: WorkbenchProps['subscribe']) {
  const ref = useRef(initial);
  const invalidate = useThree(state => state.invalidate);
  useEffect(
    () =>
      subscribe(stop => {
        ref.current = stop;
        invalidate();
      }),
    [subscribe, invalidate]
  );
  return ref;
}

function Environment() {
  const { gl, scene } = useThree();
  useEffect(() => {
    const pmrem = new THREE.PMREMGenerator(gl);
    const env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = env;
    // A dim room: metals get something to reflect without lifting the night.
    scene.environmentIntensity = 0.16;
    pmrem.dispose();
    return () => {
      scene.environment = null;
      env.dispose();
    };
  }, [gl, scene]);
  return null;
}

const nextFrame = () => new Promise<void>(done => requestAnimationFrame(() => done()));

/**
 * Gets a model ready before it joins the scene: its shaders compile in
 * parallel where the driver allows it and its textures upload, one model per
 * frame, so no single frame does every model's work at once.
 */
async function warm(
  gl: THREE.WebGLRenderer,
  root: THREE.Object3D,
  camera: THREE.Camera,
  scene: THREE.Scene
): Promise<void> {
  await gl.compileAsync(root, camera, scene);
  await nextFrame();
  root.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const materials: THREE.Material[] = Array.isArray(object.material)
      ? object.material
      : [object.material];
    for (const material of materials) {
      for (const value of Object.values(material)) {
        if (value instanceof THREE.Texture) gl.initTexture(value);
      }
    }
  });
}

function Models({
  stop,
  reducedMotion,
  firstView,
  onReady,
  onFailed,
}: {
  stop: React.RefObject<StopId>;
  reducedMotion: boolean;
  firstView: StopId;
  onReady: () => void;
  onFailed: (reason: string) => void;
}) {
  const { gl, camera, scene, invalidate } = useThree();
  const [built, setBuilt] = useState<BuiltModel[]>([]);
  const sweepClock = useRef(0);

  useEffect(() => {
    let live = true;
    const needed = new Set(MODELS.filter(m => m.stops.includes(firstView)).map(m => m.file));
    const placed: BuiltModel[] = [];
    let readySent = false;
    // Files finish loading in any order; they are warmed and placed one at a time.
    let queue = Promise.resolve();
    const place = async (file: string, scene3d: THREE.Object3D) => {
      const entries = MODELS.filter((entry: ModelEntry) => entry.file === file);
      for (const entry of entries) {
        const model = buildModel(entry, scene3d);
        await warm(gl, model.root, camera, scene);
        if (!live) return;
        placed.push(model);
      }
      // Keep manifest order so draw order is stable.
      const order = (b: BuiltModel) => MODELS.indexOf(b.entry);
      setBuilt([...placed].sort((a, b) => order(a) - order(b)));
      needed.delete(file);
      if (!readySent && needed.size === 0) {
        readySent = true;
        // Two frames: one to commit the last model, one to draw the first view.
        requestAnimationFrame(() => requestAnimationFrame(() => live && onReady()));
      }
      invalidate();
    };
    void loadInOrder(
      gl,
      loadOrder(MODELS, firstView),
      (file, gltf) => {
        if (!live) return;
        queue = queue
          .then(() => (live ? place(file, gltf.scene) : undefined))
          .catch((error: unknown) => {
            console.error(`Could not place ${file}`, error);
            if (live && needed.has(file)) onFailed(`place ${file}`);
          });
      },
      (file, error) => {
        console.error(`Could not load ${file}`, error);
        if (live && needed.has(file)) onFailed(`load ${file}`);
      }
    );
    return () => {
      live = false;
    };
  }, [gl, camera, scene, firstView, invalidate, onReady, onFailed]);

  useFrame((_, delta) => {
    const at = stop.current;
    let moving = false;
    for (const { entry, root, sweep } of built) {
      if (entry.spinAt && !reducedMotion && entry.spinAt === at) {
        root.rotation.y += delta * SPIN;
        moving = true;
      }
      if (sweep) {
        const on = entry.sweepAt === at;
        sweep.setVisible(on);
        if (on && reducedMotion) sweep.setProgress(0.55);
        else if (on) {
          sweepClock.current = (sweepClock.current + delta) % (SWEEP_PASS + SWEEP_HOLD);
          const t = Math.min(1, sweepClock.current / SWEEP_PASS);
          sweep.setProgress(t * t * (3 - 2 * t));
          moving = true;
        } else sweepClock.current = 0;
      }
    }
    if (moving) invalidate();
  });

  return (
    <>
      {built.map(model => (
        <primitive key={model.entry.id} object={model.root} />
      ))}
    </>
  );
}

function Scene(props: WorkbenchProps) {
  const stop = useStop(props.initialStop, props.subscribe);
  return (
    <>
      <color attach="background" args={['#1a1917']} />
      <Environment />
      <Lights />
      <Models
        stop={stop}
        reducedMotion={props.reducedMotion}
        firstView={props.initialStop}
        onReady={props.onReady}
        onFailed={props.onFailed}
      />
      <CameraRig
        stop={stop}
        stops={STOPS}
        reducedMotion={props.reducedMotion}
        fixed={
          props.capture && props.capture !== 'stops' ? POSTER_CAMERAS[props.capture] : undefined
        }
      />
      <Callouts stop={stop} />
    </>
  );
}

export function Workbench(props: WorkbenchProps) {
  const start = STOPS.find(s => s.id === props.initialStop) ?? STOPS[0];
  return (
    <Canvas
      frameloop="demand"
      dpr={[1, 1.5]}
      shadows="percentage"
      camera={{ fov: start?.fov ?? 36, near: 0.05, far: 30, position: start?.position }}
      gl={{ antialias: true, powerPreference: 'high-performance', stencil: false }}
      onCreated={({ gl }) => {
        if (props.capture) Object.assign(window, { __maquetteRenderer: gl });
        gl.toneMapping = THREE.ACESFilmicToneMapping;
        gl.toneMappingExposure = 1.15;
        gl.domElement.addEventListener('webglcontextlost', () => props.onFailed('context lost'));
      }}
      aria-hidden="true"
      tabIndex={-1}
    >
      <Scene {...props} />
    </Canvas>
  );
}
