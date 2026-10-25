import { useEffect } from 'react';
import * as THREE from 'three';
import type { CompareRig } from './Compare.types';

/** A click (not a drag) picks the model under the pointer, on its side. */
export function useComparePick(rig: CompareRig): void {
  const { gl, camera, raycaster, left, right, live } = rig;
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
      p.handlers.onPick((node?.userData.modelId as string | undefined) ?? null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, raycaster, left, right, live]);
}
