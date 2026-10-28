import { useEffect, useEffectEvent, useMemo, useRef, useState } from 'react';
import { useThree } from '@react-three/fiber';
import type * as THREE from 'three';
import { disposeModel } from '../../dispose';
import { originalsOf } from '../../materials';
import { ROOT_FLAG } from '../../parts';
import {
  castShadows,
  frameCamera,
  gltfLoader,
  standOnFloor,
  studioRig,
  type Framing,
} from '../../studio';
import type { Loaded, ModelSource } from '../ModelPreview.types';

/**
 * Loads the file at `url`, again whenever it changes; null while there is
 * none. The model it replaces is freed.
 */
export function useModel(url: string | null, onError: ModelSource['onError']): Loaded | null {
  const gl = useThree(s => s.gl);
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const fail = useEffectEvent((message: string) => onError?.(message));
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
        scene.userData[ROOT_FLAG] = true;
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

/**
 * Stands a newly loaded model on the floor and frames the camera on it, and
 * tells `onLoaded` once per model.
 */
export function useFraming(
  loaded: Loaded | null,
  onLoaded: ModelSource['onLoaded']
): Framing | null {
  const camera = useThree(s => s.camera) as THREE.PerspectiveCamera;
  const [framing, setFraming] = useState<Framing | null>(null);
  const reported = useRef<Loaded | null>(null);
  const report = useEffectEvent((model: Loaded) =>
    onLoaded?.({ meshes: model.originals.size, bones: model.bones.length })
  );
  useEffect(() => {
    if (!loaded) return;
    const next = standOnFloor(loaded.scene);
    if (!next) return;
    frameCamera(camera, next);
    setFraming(next);
    if (reported.current === loaded) return;
    reported.current = loaded;
    report(loaded);
  }, [loaded, camera]);
  return framing;
}

/** The studio's lights and floor for `framing`, disposed when it is replaced. */
export function useStudioRig(framing: Framing | null): THREE.Group | null {
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
  return rig;
}
