import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef, useState } from 'react';
import type * as THREE from 'three';
import { buildModel, disposeScreens, type BuiltModel } from './build';
import { disposeBuilt, fileGeometries } from './dispose';
import { disposeFiles, disposeLoaders, loadInOrder } from './loader';
import { MODELS, STOP_IDS, TIMELINE, loadOrder, type ModelEntry, type StopId } from './manifest';
import { across, presence, smoothstep } from './progress';
import { rigTracks, type Tracks } from './tracks';
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

/** A placed model with the scroll timeline's tracks, when it has any. */
export type PlacedModel = BuiltModel & { tracks: Tracks | null };

/**
 * Loads, warms and places the manifest's models (the first view's first),
 * then sets the turntable's turn, the wireframe sweep and the scroll
 * timeline's tracks from the tour progress each frame: they follow the
 * scroll, and the turntable also idles round on its own. On unmount it frees
 * the files, the clones and the loaders.
 *
 * Shadow maps only render on frames where a model that casts moved (or one
 * was placed): three would otherwise redraw them every frame. Only the
 * manifest's models cast shadows, and the lights stand still.
 */
export function useModels({ progress, still, firstView, onReady, onFailed }: ModelsOptions) {
  const { gl, camera, scene, invalidate } = useThree();
  const [built, setBuilt] = useState<PlacedModel[]>([]);
  const last = useRef({ built, at: Number.NaN, playing: false });

  useEffect(() => {
    gl.shadowMap.autoUpdate = false;
    gl.shadowMap.needsUpdate = true;
    return () => {
      gl.shadowMap.autoUpdate = true;
    };
  }, [gl]);

  useEffect(() => {
    let live = true;
    const needed = new Set(MODELS.filter(m => m.stops.includes(firstView)).map(m => m.file));
    const placed: PlacedModel[] = [];
    let readySent = false;
    // Files finish loading in any order; they are warmed and placed one at a time.
    let queue = Promise.resolve();
    const place = async (file: string, scene3d: THREE.Object3D) => {
      const entries = MODELS.filter((entry: ModelEntry) => entry.file === file);
      for (const entry of entries) {
        const model = buildModel(entry, scene3d);
        await warm(gl, model.root, camera, scene);
        // Torn down while it warmed: its file is already freed, the clone is not.
        if (!live) return disposeBuilt(model.root, fileGeometries(scene3d));
        placed.push({ ...model, tracks: rigTracks(model, TIMELINE) });
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
      },
      () => live
    );
    return () => {
      live = false;
      // Each file once (its geometries and textures), then the clones' own
      // materials, then the loaders. A clone never frees a shared texture.
      const shared = disposeFiles();
      for (const model of placed) disposeBuilt(model.root, shared);
      disposeScreens();
      disposeLoaders();
    };
  }, [gl, camera, scene, firstView, invalidate, onReady, onFailed]);

  useFrame(state => {
    const at = progress.current;
    const now = state.clock.elapsedTime;
    const idle = still ? 0 : now * IDLE_TURN;
    let moving = false;
    let shadows = built !== last.current.built;
    // Scrubbed poses follow the progress; a timed move still poses the frame after it ends.
    const posed = at !== last.current.at || last.current.playing;
    for (const { entry, root, sweep, tracks } of built) {
      const casts = entry.castShadow ?? false;
      if (tracks) {
        const playing = tracks.apply(at, now);
        if (playing) moving = true;
        if (casts && (playing || posed)) shadows = true;
      }
      if (entry.spinAt) {
        const stop = stopIndex(entry.spinAt);
        const turn = across(at, stop - TURN, stop + TURN) * Math.PI * 2 + idle;
        if (casts && turn !== root.rotation.y) shadows = true;
        root.rotation.y = turn;
      }
      if (sweep && entry.sweepAt) {
        const stop = stopIndex(entry.sweepAt);
        sweep.setOpacity(presence(at, stop));
        sweep.setProgress(smoothstep(at, stop - SWEEP, stop + SWEEP));
      }
    }
    last.current = { built, at, playing: moving };
    if (shadows) gl.shadowMap.needsUpdate = true;
    // A stop change plays over time, so it asks for the frames it needs.
    if (moving) state.invalidate();
  });

  return built;
}
