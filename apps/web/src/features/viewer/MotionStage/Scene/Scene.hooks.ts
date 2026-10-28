import { useEffect, useEffectEvent, useMemo, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { disposeModel } from '../../dispose';
import { originalsOf } from '../../materials';
import {
  applyOffsets,
  clipPlayer,
  restPoses,
  rootOf,
  rootTrail,
  trailLine,
  type PartOffset,
} from '../../motion-pose';
import { BACKDROP, CLAY, castShadows, frameCamera, gltfLoader, type Framing } from '../../studio';
import type { LoadedMotion, SceneProps } from './Scene.types';

/**
 * Loads the file at `url` with its clips, again whenever it changes; null
 * while there is none. The model it replaces is freed.
 */
export function useMotionModel(
  url: string | null,
  onError: SceneProps['onError']
): LoadedMotion | null {
  const gl = useThree(s => s.gl);
  const scene = useThree(s => s.scene);
  const [loaded, setLoaded] = useState<LoadedMotion | null>(null);
  const fail = useEffectEvent((message: string) => onError?.(message));
  useEffect(() => {
    scene.background = new THREE.Color(BACKDROP);
  }, [scene]);
  useEffect(() => {
    setLoaded(null);
    if (!url) return;
    let alive = true;
    gltfLoader(gl).load(
      url,
      gltf => {
        if (!alive) return;
        const bones: THREE.Bone[] = [];
        gltf.scene.traverse(o => {
          if ((o as THREE.Bone).isBone) bones.push(o as THREE.Bone);
        });
        castShadows(gltf.scene);
        setLoaded({
          scene: gltf.scene,
          originals: originalsOf(gltf.scene),
          bones,
          clips: gltf.animations ?? [],
        });
      },
      undefined,
      err => {
        if (alive) fail(err instanceof Error ? err.message : 'The model did not load.');
      }
    );
    return () => {
      alive = false;
    };
  }, [url, gl]);
  // A model replaced by the next file, or left on unmount, frees its GPU memory.
  useEffect(
    () => () => {
      if (loaded) disposeModel(loaded);
    },
    [loaded]
  );
  return loaded;
}

/** Moves the named parts by their state's offsets, and back to rest without one. */
export function usePoses(
  loaded: LoadedMotion | null,
  poses: Record<string, PartOffset> | null
): void {
  const parts = useMemo(() => (loaded ? restPoses(loaded.scene) : null), [loaded]);
  useEffect(() => {
    if (parts) applyOffsets(parts, poses);
  }, [parts, poses]);
}

/** `framing` grown to hold the model wherever its root stands along `points`. */
export function travelFraming(framing: Framing, points: readonly THREE.Vector3[]): Framing {
  const size = framing.box.getSize(new THREE.Vector3());
  const half = new THREE.Vector3(size.x / 2, 0, size.z / 2);
  const box = framing.box.clone();
  const start = points[0];
  for (const p of points) {
    const shift = start ? p.clone().sub(start) : new THREE.Vector3();
    box.expandByPoint(framing.target.clone().add(shift).add(half));
    box.expandByPoint(framing.target.clone().add(shift).sub(half));
  }
  const target = box.getCenter(new THREE.Vector3());
  target.y = framing.target.y;
  return { box, extent: box.getSize(new THREE.Vector3()).length() || framing.extent, target };
}

/**
 * Plays clip `clip` at `time`, frames the camera on the whole walk of a
 * clip whose root travels, and with `trail` on returns the dashed path its
 * root walks. The camera target comes back with it for the orbit controls.
 */
export function useClip(
  loaded: LoadedMotion | null,
  { clip, time, trail }: Pick<SceneProps, 'clip' | 'time' | 'trail'>,
  framing: Framing | null
): { line: THREE.Line | null; target: THREE.Vector3 | null } {
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const animation = loaded && clip !== null ? (loaded.clips[clip] ?? null) : null;
  const player = useMemo(
    () => (loaded && animation ? clipPlayer(loaded.scene, animation) : null),
    [loaded, animation]
  );
  useEffect(() => () => player?.dispose(), [player]);
  const points = useMemo(() => {
    if (!loaded || !animation || !player) return null;
    const root = rootOf(loaded.scene, animation);
    if (!root) return null;
    const out = rootTrail(player, loaded.scene, root, animation.duration);
    return out[0]!.distanceTo(out[out.length - 1]!) > 0.01 ? out : null;
  }, [loaded, animation, player]);
  const framed = useMemo(
    () => (framing && points ? travelFraming(framing, points) : framing),
    [framing, points]
  );
  useEffect(() => {
    if (framed && points) frameCamera(camera, framed);
  }, [camera, framed, points]);
  const extent = framing?.extent ?? 1;
  const line = useMemo(
    () => (points && trail ? trailLine(points, '#e9e5dc', extent * 0.002) : null),
    [points, trail, extent]
  );
  useEffect(
    () => () => {
      line?.geometry.dispose();
      (line?.material as THREE.Material | undefined)?.dispose();
    },
    [line]
  );
  useEffect(() => {
    player?.seek(time);
  }, [player, time, points]);
  return { line, target: framed?.target ?? null };
}

/**
 * The floor grid and a box around each part in `outline`, kept on the
 * moving parts every frame; also hands each frame's time to `onFrame`.
 */
export function useOverlays(
  loaded: LoadedMotion | null,
  outline: readonly string[],
  extent: number,
  onFrame: SceneProps['onFrame']
): THREE.Group | null {
  const group = useMemo(() => {
    if (!loaded) return null;
    const out = new THREE.Group();
    const grid = new THREE.GridHelper(extent * 6, 24, '#5a554d', '#45413b');
    const material = grid.material as THREE.LineBasicMaterial;
    material.transparent = true;
    material.opacity = 0.35;
    material.depthWrite = false;
    grid.position.y = extent * 0.0008;
    out.add(grid);
    const named = restPoses(loaded.scene);
    for (const part of outline) {
      const node = named.get(part)?.node;
      if (node) out.add(new THREE.BoxHelper(node, CLAY));
    }
    return out;
  }, [loaded, outline, extent]);
  useEffect(
    () => () =>
      group?.traverse(o => {
        const line = o as THREE.LineSegments;
        if (!line.isLineSegments) return;
        line.geometry.dispose();
        (line.material as THREE.Material).dispose();
      }),
    [group]
  );
  useFrame((_, delta) => {
    group?.children.forEach(o => (o as THREE.BoxHelper).update?.());
    onFrame?.(delta);
  });
  return group;
}
