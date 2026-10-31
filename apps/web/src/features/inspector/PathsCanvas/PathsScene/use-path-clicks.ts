import { useEffect } from 'react';
import * as THREE from 'three';
import type { Vec3 } from '@/state/inspector';
import { snapPoint } from '@/state/paths';
import type { PathsRig } from './PathsScene.types';

const GROUND = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);

/**
 * A click (not a drag, not on the gizmo): with Select it picks a point of
 * the selected path, else a path, else nothing; with Add points it puts a
 * point where the click lands on a model or the ground. Draw on surfaces
 * has its own hook (./use-path-stroke.ts).
 */
export function usePathClicks(rig: PathsRig): void {
  const { gl, camera, raycaster, world, tubes, handles, transform, live } = rig;
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
      const p = live.current;
      if (p.tool === 'surface') return;
      if (p.tool === 'add') {
        const hit = raycaster.intersectObject(world, true)[0]?.point;
        const ground = hit ?? raycaster.ray.intersectPlane(GROUND, new THREE.Vector3());
        if (ground) p.handlers.onAddPoint(snapPoint(ground.toArray() as Vec3, p.snap));
        return;
      }
      const handle = raycaster.intersectObject(handles, true)[0]?.object;
      const path = p.selection?.path;
      if (handle && path) {
        p.handlers.onSelect({ path, point: handle.userData.point as number });
        return;
      }
      const tube = raycaster.intersectObject(tubes, true)[0]?.object;
      if (tube) {
        p.handlers.onSelect({ path: tube.userData.path as string, point: null });
        return;
      }
      p.handlers.onSelect(path ? { path, point: null } : null);
    };
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointerup', onUp);
    };
  }, [gl, camera, raycaster, world, tubes, handles, transform, live]);
}
