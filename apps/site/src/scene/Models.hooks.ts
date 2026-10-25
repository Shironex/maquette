import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useState } from 'react';
import type * as THREE from 'three';
import { buildModel, type BuiltModel } from './build';
import { loadInOrder } from './loader';
import { MODELS, STOP_IDS, loadOrder, type ModelEntry, type StopId } from './manifest';
import { across, presence, smoothstep } from './progress';
import { warm } from './warm';

/**
 * The turntable makes one full turn while progress is this many stops
 * either side of its stop, so it faces the same way before and after.
 */
const TURN = 0.45;
/** The wireframe sweep crosses the model while progress is this many stops either side of its stop. */
const SWEEP = 0.34;
/** The turntable's own slow turn at rest, in radians per second. */
const IDLE_TURN = 0.14;

export interface ModelsOptions {
  /** Tour progress in stops. */
  progress: React.RefObject<number>;
  /** Poster capture: no idle turn, so every poster is the same frame. */
  still: boolean;
  firstView: StopId;
  onReady: () => void;
  onFailed: (reason: string) => void;
}

const stopIndex = (id: StopId) => STOP_IDS.indexOf(id);

/**
 * Loads, warms and places the manifest's models (the first view's first),
 * then sets the turntable's turn and the wireframe sweep from the tour
 * progress each frame: both follow the scroll, and the turntable also
 * idles round on its own.
 */
export function useModels({ progress, still, firstView, onReady, onFailed }: ModelsOptions) {
  const { gl, camera, scene, invalidate } = useThree();
  const [built, setBuilt] = useState<BuiltModel[]>([]);

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
            console.error('Could not place a model', { file, error });
            if (live && needed.has(file)) onFailed(`place ${file}`);
          });
      },
      (file, error) => {
        console.error('Could not load a model', { file, error });
        if (live && needed.has(file)) onFailed(`load ${file}`);
      }
    );
    return () => {
      live = false;
    };
  }, [gl, camera, scene, firstView, invalidate, onReady, onFailed]);

  useFrame(state => {
    const at = progress.current;
    const idle = still ? 0 : state.clock.elapsedTime * IDLE_TURN;
    for (const { entry, root, sweep } of built) {
      if (entry.spinAt) {
        const stop = stopIndex(entry.spinAt);
        root.rotation.y = across(at, stop - TURN, stop + TURN) * Math.PI * 2 + idle;
      }
      if (sweep && entry.sweepAt) {
        const stop = stopIndex(entry.sweepAt);
        sweep.setOpacity(presence(at, stop));
        sweep.setProgress(smoothstep(at, stop - SWEEP, stop + SWEEP));
      }
    }
  });

  return built;
}
