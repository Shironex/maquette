import { useEffect } from 'react';
import * as THREE from 'three';
import type { EditorRig } from './Editor.types';

/** A click (not a drag, not on the gizmo) selects what is under the pointer. */
export function useClickSelect(editor: EditorRig): void {
  const { gl, camera, raycaster, world, stopsGroup, transform, live } = editor;
  useEffect(() => {
    const el = gl.domElement;
    let down: { x: number; y: number } | null = null;
    const onDown = (e: PointerEvent) => {
      down = { x: e.clientX, y: e.clientY };
    };
    const onUp = (e: PointerEvent) => {
      if (!down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
      down = null;
      if (transform.dragging || transform.axis) return;
      const rect = el.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(ndc, camera);
      const targets = [world, ...(stopsGroup.visible ? [stopsGroup] : [])];
      const hits = raycaster.intersectObjects(targets, true);
      for (const hit of hits) {
        let node: THREE.Object3D | null = hit.object;
        while (node && node.userData.asset === undefined && node.userData.stop === undefined) {
          node = node.parent;
        }
        if (node?.userData.stop !== undefined && hit.object.name === 'stop-body') {
          live.current.handlers.onSelect({ kind: 'stop', id: node.userData.stop as string });
          return;
        }
        if (node?.userData.asset !== undefined) {
          live.current.handlers.onSelect({ kind: 'model', id: node.userData.asset as string });
          return;
        }
      }
      live.current.handlers.onSelect(null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, raycaster, world, stopsGroup, transform, live]);
}
