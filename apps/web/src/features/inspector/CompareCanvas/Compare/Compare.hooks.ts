import { useEffect, useEffectEvent, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { clone as cloneSkinned } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { checkerTexture, originalsOf } from '@/features/viewer/materials';
import { ROOT_FLAG } from '@/features/viewer/parts';
import { BACKDROP, castShadows, gltfLoader, type Framing } from '@/features/viewer/studio';
import { FLIGHT_MS, SAMPLE_EVERY_MS, flight } from '@/state/inspector';
import { setPose } from '../../camera-pose';
import type { CompareModel } from '../CompareCanvas.types';
import type { CompareProps, CompareRig, Side } from './Compare.types';
import { disposeObject } from '@/features/viewer/dispose';
import { shootStops } from './shoot-stops';
import { useComparePick } from './use-compare-pick';
import { useCompareView } from './use-compare-view';

/** Frames between two timed (GPU synced) frames. */
const TIME_EVERY = 4;

/** Loads every model of one side, each file once, placed at its position. */
export function useSide(models: CompareModel[], onError: (message: string) => void): Side | null {
  const gl = useThree(s => s.gl);
  const [side, setSide] = useState<Side | null>(null);
  // Only different models load again, not a new array of the same.
  const key = JSON.stringify(models);
  const current = useEffectEvent(() => models);
  const fail = useEffectEvent(onError);
  useEffect(() => {
    const list = current();
    if (list.length === 0) {
      setSide(null);
      return;
    }
    let alive = true;
    let made: THREE.Group | null = null;
    const loader = gltfLoader(gl);
    const urls = [...new Set(list.map(m => m.url))];
    Promise.all(urls.map(url => loader.loadAsync(url)))
      .then(loaded => {
        if (!alive) return;
        const byUrl = new Map(urls.map((url, i) => [url, loaded[i]!]));
        const used = new Set<string>();
        const group = new THREE.Group();
        const mixers: THREE.AnimationMixer[] = [];
        for (const model of list) {
          const gltf = byUrl.get(model.url)!;
          // A file placed twice needs its own copy, skeleton included.
          const scene = used.has(model.url)
            ? (cloneSkinned(gltf.scene) as THREE.Group)
            : gltf.scene;
          used.add(model.url);
          scene.userData[ROOT_FLAG] = true;
          const holder = new THREE.Group();
          holder.position.set(...model.position);
          holder.userData.modelId = model.id;
          holder.userData.asset = model.asset ?? null;
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
        if (alive) fail(err instanceof Error ? err.message : 'A model did not load.');
      });
    return () => {
      alive = false;
      if (made) disposeObject(made);
    };
  }, [key, gl]);
  return side;
}

/** Both sides, their two scenes, the orbit and the refs the compare hooks share. */
export function useCompareRig(props: CompareProps): CompareRig {
  const { gl, size } = useThree();
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const left = useSide(props.left, props.handlers.onError);
  const right = useSide(props.right, props.handlers.onError);
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
  const flightRef: CompareRig['flightRef'] = useRef(null);
  const shotsPending = useRef(false);
  const stats = useRef({ windowStart: 0, frames: 0, timed: 0, work: 0, last: 0 });
  const pixel = useMemo(() => new Uint8Array(4), []);
  const raycaster = useMemo(() => new THREE.Raycaster(), []);
  return {
    gl,
    size,
    camera,
    left,
    right,
    scenes,
    controls,
    checker,
    framing,
    setFraming,
    live,
    flightRef,
    shotsPending,
    stats,
    pixel,
    raycaster,
  };
}

/**
 * Each frame: the animations, the camera's flight or orbit, the stops'
 * live renders when they wait, both sides scissored at the split, and the
 * frame time sample.
 */
export function useCompareFrame(rig: CompareRig): void {
  const { gl, size, camera, left, right, scenes, controls, framing } = rig;
  const { live, flightRef, shotsPending, stats, pixel } = rig;
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
      const shots = shootStops(gl, camera, size, scenes.right, p.stops);
      if (shots) p.handlers.onStopShots(shots);
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
    if (span < SAMPLE_EVERY_MS) return;
    if (s.timed > 0) s.last = s.work / s.timed;
    if (s.last > 0) p.handlers.onSample(s.last, (s.frames * 1000) / span);
    s.windowStart = now;
    s.frames = 0;
    s.timed = 0;
    s.work = 0;
  }, 1);
}

/**
 * The whole compare view, in the order its effects must run: the sides and
 * the camera, the looks, click to pick and the frame loop.
 */
export function useCompare(props: CompareProps): void {
  const rig = useCompareRig(props);
  useCompareView(rig, props);
  useComparePick(rig);
  useCompareFrame(rig);
}
