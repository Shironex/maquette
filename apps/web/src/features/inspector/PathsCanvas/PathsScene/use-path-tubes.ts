import { useEffect } from 'react';
import * as THREE from 'three';
import { placementOf } from '@/state/layout';
import { BACKDROP } from '@/features/viewer/studio';
import { disposeObject } from '@/features/viewer/dispose';
import { applyPlacement, applyPrep } from '../../LayoutCanvas/Editor/placement';
import type { PathsRig, PathsSceneProps } from './PathsScene.types';
import { pathTube } from './path-tube';

const LINE = '#4d4a44';

const clear = (group: THREE.Group) => {
  for (const child of [...group.children]) {
    group.remove(child);
    disposeObject(child);
  }
};

/**
 * The scene: backdrop, lights, a ground grid, the models where the layout
 * puts them and a tube for every path.
 */
export function usePathTubes(rig: PathsRig, props: PathsSceneProps): void {
  const { scene, holders, live, transform, world, tubes, handles } = rig;

  useEffect(() => {
    scene.background = new THREE.Color(BACKDROP);
    const lights = new THREE.Group();
    lights.add(new THREE.HemisphereLight('#f4efe6', '#2a2926', 1.1));
    const key = new THREE.DirectionalLight('#ffd9b5', 2.2);
    key.position.set(-6, 10, 7);
    const rim = new THREE.DirectionalLight('#b8ccff', 1);
    rim.position.set(2, 8, -10);
    lights.add(key, rim);
    const grid = new THREE.GridHelper(24, 48, LINE, LINE);
    grid.position.y = 0.0015;
    const material = grid.material as THREE.Material;
    material.transparent = true;
    material.opacity = 0.55;
    material.depthWrite = false;
    lights.add(grid);
    scene.add(lights, world, tubes, handles, transform.getHelper());
    return () => {
      scene.remove(lights, world, tubes, handles, transform.getHelper());
      disposeObject(lights);
    };
  }, [scene, world, tubes, handles, transform]);

  useEffect(() => {
    if (!holders) return;
    for (const holder of holders.values()) world.add(holder);
    live.current.handlers.onReady();
    return () => {
      for (const holder of holders.values()) world.remove(holder);
    };
  }, [holders, world, live]);

  useEffect(() => {
    if (!holders) return;
    for (const model of props.models) {
      const holder = holders.get(model.asset);
      if (!holder) continue;
      applyPrep(holder, model.prep);
      applyPlacement(holder, placementOf(props.layout, model));
    }
  }, [holders, props.models, props.layout]);

  // The tubes are cheap, so every change builds them again.
  const selected = props.selection?.path ?? null;
  useEffect(() => {
    for (const path of props.paths) {
      const tube = pathTube(path, path.id === selected);
      if (tube) tubes.add(tube);
    }
    return () => clear(tubes);
  }, [props.paths, selected, tubes]);
}
