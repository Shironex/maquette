import { useEffect } from 'react';
import * as THREE from 'three';
import type { Vec3 } from '@/state/inspector';
import { DEFAULT_WIDTH } from '@/state/paths';
import { STROKE_SPACING } from '@/state/path-stroke';
import type { PathsRig } from './PathsScene.types';

const INK = '#f4efe6';
/** Samples kept for the line under a drag, far more than a stroke leaves on a path. */
const MAX_SAMPLES = 2048;

/**
 * Draw on surfaces: a drag that starts on a model follows it, sampling where
 * the pointer lands, lifted off the surface by the tube's radius so the tube
 * lies on it. Letting go hands the stroke to the editor; a drag that starts
 * off the models, or on the gizmo, is left to them.
 */
export function usePathStroke(rig: PathsRig): void {
  const { gl, scene, camera, raycaster, world, orbit, transform, live } = rig;
  useEffect(() => {
    const el = gl.domElement;
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array(MAX_SAMPLES * 3);
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const line = new THREE.Line(
      geometry,
      new THREE.LineBasicMaterial({ color: INK, depthTest: false, transparent: true })
    );
    line.name = 'path-stroke';
    line.frustumCulled = false;
    line.renderOrder = 11;
    line.visible = false;
    scene.add(line);

    const pointer = new THREE.Vector2();
    const normal = new THREE.Vector3();
    const toWorld = new THREE.Matrix3();
    const landing = new THREE.Vector3();
    let samples: Vec3[] | null = null;
    let lift = DEFAULT_WIDTH / 2;

    /** Where the pointer lands on a model, lifted along its surface. */
    const land = (e: PointerEvent): Vec3 | null => {
      const rect = el.getBoundingClientRect();
      pointer.set(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1
      );
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObject(world, true)[0];
      if (!hit) return null;
      landing.copy(hit.point);
      if (hit.face) {
        toWorld.getNormalMatrix(hit.object.matrixWorld);
        normal.copy(hit.face.normal).applyMatrix3(toWorld).normalize();
        landing.addScaledVector(normal, lift);
      }
      return landing.toArray() as Vec3;
    };

    const push = (point: Vec3) => {
      if (!samples || samples.length >= MAX_SAMPLES) return;
      const last = samples.at(-1);
      if (
        last &&
        Math.hypot(point[0] - last[0], point[1] - last[1], point[2] - last[2]) < STROKE_SPACING
      ) {
        return;
      }
      positions.set(point, samples.length * 3);
      samples.push(point);
      geometry.setDrawRange(0, samples.length);
      geometry.attributes.position!.needsUpdate = true;
    };

    const onDown = (e: PointerEvent) => {
      const p = live.current;
      if (p.tool !== 'surface' || e.button !== 0 || transform.axis || transform.dragging) return;
      const path = p.paths.find(q => q.id === p.selection?.path);
      if (!path) return;
      lift = (path.width ?? DEFAULT_WIDTH) / 2;
      const start = land(e);
      if (!start) return;
      samples = [];
      push(start);
      line.visible = true;
      orbit.enabled = false;
    };

    const onMove = (e: PointerEvent) => {
      if (!samples) return;
      const point = land(e);
      if (point) push(point);
    };

    const finish = (commit: boolean) => {
      if (!samples) return;
      const stroke = samples;
      samples = null;
      line.visible = false;
      geometry.setDrawRange(0, 0);
      orbit.enabled = true;
      if (commit && stroke.length > 0) live.current.handlers.onStroke(stroke);
    };
    const onUp = () => finish(true);
    const onCancel = () => finish(false);

    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointermove', onMove);
    el.addEventListener('pointerup', onUp);
    el.addEventListener('pointercancel', onCancel);
    return () => {
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointermove', onMove);
      el.removeEventListener('pointerup', onUp);
      el.removeEventListener('pointercancel', onCancel);
      scene.remove(line);
      geometry.dispose();
      line.material.dispose();
      orbit.enabled = true;
    };
  }, [gl, scene, camera, raycaster, world, orbit, transform, live]);
}
