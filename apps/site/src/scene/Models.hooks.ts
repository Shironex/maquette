import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef, useState } from 'react';
import type * as THREE from 'three';
import { buildModel, type BuiltModel } from './build';
import { loadInOrder } from './loader';
import { MODELS, loadOrder, type ModelEntry, type StopId } from './manifest';
import { warm } from './warm';

/** Spin speed of the turntable, radians per second. */
const SPIN = 0.32;
/** One pass of the wireframe sweep, then a short hold, in seconds. */
const SWEEP_PASS = 2.6;
const SWEEP_HOLD = 1.2;

export interface ModelsOptions {
  stop: React.RefObject<StopId>;
  reducedMotion: boolean;
  firstView: StopId;
  onReady: () => void;
  onFailed: (reason: string) => void;
}

/**
 * Loads, warms and places the manifest's models (the first view's first),
 * then drives the turntable spin and the wireframe sweep each frame.
 */
export function useModels({ stop, reducedMotion, firstView, onReady, onFailed }: ModelsOptions) {
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

  return built;
}
