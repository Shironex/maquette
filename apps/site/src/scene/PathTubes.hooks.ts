import { useFrame } from '@react-three/fiber';
import { useEffect, useMemo } from 'react';
import * as THREE from 'three';
import { PATHS, PATH_TRACKS, TIMELINE } from './manifest';
import { buildPath, energize } from './path-mesh';
import { energizeAt } from './paths';

/**
 * The shipped paths as tubes in one group, lit by their energize tracks
 * from the tour progress each frame. One draw call per path; nothing runs
 * when the project has none.
 */
export function usePathTubes(progress: React.RefObject<number>) {
  const built = useMemo(() => {
    const group = new THREE.Group();
    group.name = 'paths';
    const meshes = PATHS.map(path => ({ path, mesh: buildPath(path) }));
    for (const { mesh } of meshes) group.add(mesh);
    return { group, meshes };
  }, []);

  useEffect(
    () => () => {
      for (const { mesh } of built.meshes) {
        mesh.geometry.dispose();
        mesh.material.dispose();
      }
    },
    [built]
  );

  useFrame(() => {
    const at = progress.current;
    for (const { path, mesh } of built.meshes) {
      energize(mesh, energizeAt(PATH_TRACKS[path.id], TIMELINE.stopIds, at));
    }
  });

  return built.group;
}
