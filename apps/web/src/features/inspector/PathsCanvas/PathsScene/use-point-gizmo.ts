import { useEffect } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { CLAY } from '@/features/viewer/studio';
import type { Vec3 } from '@/state/inspector';
import { SNAP } from '@/state/layout';
import { pointsOf } from '@/state/paths';
import { disposeTree } from '../../LayoutCanvas/Editor/dispose-tree';
import type { PathsRig, PathsSceneProps } from './PathsScene.types';

const HANDLE = '#f4efe6';
const ANCHOR = '#8fa58c';
/** A handle's radius per metre of camera distance, so it keeps its size on screen. */
const HANDLE_SIZE = 0.012;

/**
 * A handle on each point of the selected path, three's transform gizmo on
 * the picked one, and what its drags change. The handles are built again
 * only when the path, its point count or the pick changes, so a drag keeps
 * its gizmo; their positions follow every edit.
 */
export function usePointGizmo(rig: PathsRig, props: PathsSceneProps): void {
  const { camera, live, orbit, transform, handles } = rig;
  const sel = props.selection;
  const path = sel ? props.paths.find(p => p.id === sel.path) : undefined;
  // What the handles are built from; a moved point only moves them.
  const shape = path
    ? [path.id, path.points.length, path.start, path.end, sel?.point].join('|')
    : '';

  useEffect(() => {
    const p = live.current;
    const sel = p.selection;
    const path = sel ? p.paths.find(q => q.id === sel.path) : undefined;
    if (!path) return;
    const last = path.points.length - 1;
    const geometry = new THREE.SphereGeometry(1, 16, 12);
    pointsOf(path).forEach((point, i) => {
      const end = (i === 0 && path.start) || (i === last && path.end);
      const material = new THREE.MeshBasicMaterial({
        color: sel?.point === i ? CLAY : end ? ANCHOR : HANDLE,
        depthTest: false,
        transparent: true,
      });
      const handle = new THREE.Mesh(i === 0 ? geometry : geometry.clone(), material);
      handle.name = 'path-point';
      handle.renderOrder = 10;
      handle.userData.point = i;
      handle.position.set(...point);
      handles.add(handle);
    });
    const picked = handles.children.find(h => h.userData.point === sel?.point);
    if (picked) transform.attach(picked);
    return () => {
      transform.detach();
      for (const child of [...handles.children]) {
        handles.remove(child);
        disposeTree(child);
      }
    };
  }, [shape, handles, transform, live]);

  useEffect(() => {
    if (!path || transform.dragging) return;
    const points = pointsOf(path);
    for (const handle of handles.children) {
      const point = points[handle.userData.point as number];
      if (point) handle.position.set(...point);
    }
  }, [path, handles, transform]);

  useEffect(() => {
    transform.setMode('translate');
    transform.setSpace('world');
    transform.setTranslationSnap(props.snap ? SNAP.move : null);
    transform.setSize(0.8);
  }, [transform, props.snap]);

  useEffect(() => {
    const at = () => transform.object?.position.toArray() as Vec3 | undefined;
    const index = () => transform.object?.userData.point as number | undefined;
    const onDragging = (e: { value: unknown }) => {
      orbit.enabled = !e.value;
      const i = index();
      const point = at();
      if (!e.value && i !== undefined && point) live.current.handlers.onDropPoint(i, point);
    };
    const onChange = () => {
      const i = index();
      const point = at();
      if (i !== undefined && point) live.current.handlers.onDragPoint(i, point);
    };
    transform.addEventListener('dragging-changed', onDragging);
    transform.addEventListener('objectChange', onChange);
    return () => {
      transform.removeEventListener('dragging-changed', onDragging);
      transform.removeEventListener('objectChange', onChange);
      transform.detach();
      transform.dispose();
    };
  }, [transform, orbit, live]);

  const at = new THREE.Vector3();
  useFrame(() => {
    for (const handle of handles.children) {
      const far = camera.position.distanceTo(handle.getWorldPosition(at));
      handle.scale.setScalar(Math.max(0.01, far * HANDLE_SIZE));
    }
  });
}
