import { useEffect, useMemo, useRef, useState } from 'react';
import { useFrame, type ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import type { RecipeRig } from '../recipe-pose';
import { modelBasis } from '../recipe-pose';
import { parentTurn } from '../recipe-keys-pose';
import { markDragged, useDots, useRings } from './gizmo-look';
import {
  keyAxis,
  keyFrame,
  onRingPlane,
  ringAngle,
  ringAxes,
  type GizmoSpace,
} from '../turn-gizmo';

/** A turn the gizmo asks for: round `axis` (in the key's own frame), `angle` radians since the drag began. */
export interface GizmoTurn {
  phase: 'start' | 'move' | 'end';
  axis: THREE.Vector3;
  angle: number;
}

interface Drag {
  index: number;
  pointer: number;
  center: THREE.Vector3;
  normal: THREE.Vector3;
  axis: THREE.Vector3;
  from: THREE.Vector3;
}

/** Turns a ring's +Z (a torus's own axis) to `axis`. */
const facingAxis = (axis: THREE.Vector3) =>
  new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 0, 1), axis.clone().normalize());

/**
 * Keeps the rings on the picked bone and facing its axes each frame (after
 * the pose is laid), the dots on their bones, and turns a drag round a ring
 * into turns for the key.
 */
export function usePoseGizmo(
  rig: RecipeRig,
  bone: string,
  space: GizmoSpace,
  dots: readonly string[],
  onTurn: (turn: GizmoTurn) => void,
  onDragging?: (dragging: boolean) => void
) {
  const rings = useRef<(THREE.Object3D | null)[]>([]);
  const group = useRef<THREE.Group>(null);
  const dotRefs = useRef(new Map<string, THREE.Object3D>());
  const [drag, setDrag] = useState<Drag | null>(null);

  const frameOf = (b: THREE.Object3D) =>
    keyFrame(parentTurn(rigKeys(rig), b), modelBasis(rig.forward));

  useFrame(() => {
    const b = rig.bone(bone);
    const g = group.current;
    if (g) {
      g.visible = Boolean(b);
      if (b) {
        b.getWorldPosition(g.position);
        const axes = ringAxes(space, frameOf(b));
        rings.current.forEach((r, i) => r?.quaternion.copy(facingAxis(axes[i]!)));
      }
    }
    for (const name of dots) {
      const at = rig.bone(name);
      const dot = dotRefs.current.get(name);
      if (dot) {
        dot.visible = Boolean(at);
        if (at) at.getWorldPosition(dot.position);
      }
    }
  });

  const begin = (index: number) => (e: ThreeEvent<PointerEvent>) => {
    const b = rig.bone(bone);
    if (!b) return;
    e.stopPropagation();
    const frame = frameOf(b);
    const normal = ringAxes(space, frame)[index]!;
    const center = b.getWorldPosition(new THREE.Vector3());
    const from = onRingPlane(e.ray, center, normal) ?? e.point.clone();
    (e.target as Element | null)?.setPointerCapture?.(e.pointerId);
    const axis = keyAxis(space, index, frame);
    setDrag({ index, pointer: e.pointerId, center, normal, axis, from });
    onDragging?.(true);
    onTurn({ phase: 'start', axis, angle: 0 });
  };

  const move = (e: ThreeEvent<PointerEvent>) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    e.stopPropagation();
    const to = onRingPlane(e.ray, drag.center, drag.normal);
    if (to)
      onTurn({
        phase: 'move',
        axis: drag.axis,
        angle: ringAngle(drag.center, drag.normal, drag.from, to),
      });
  };

  const end = (e: ThreeEvent<PointerEvent>) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    e.stopPropagation();
    (e.target as Element | null)?.releasePointerCapture?.(e.pointerId);
    setDrag(null);
    onDragging?.(false);
    onTurn({ phase: 'end', axis: drag.axis, angle: 0 });
  };

  const dotRef = (name: string) => (o: THREE.Object3D | null) => {
    if (o) dotRefs.current.set(name, o);
    else dotRefs.current.delete(name);
  };

  return { group, rings, dragging: drag?.index ?? null, begin, move, end, dotRef };
}

function rigKeys(rig: RecipeRig) {
  return { bone: rig.bone, forward: rig.forward, rest: rig.rest ?? (() => undefined) };
}

/**
 * The gizmo's objects wired to its drag: the rings (in a group on the
 * picked bone) and the bone dots, made once per size and kept in step.
 */
export function useGizmoObjects(
  g: ReturnType<typeof usePoseGizmo>,
  size: number,
  dots: readonly string[],
  bone: string,
  keyed: readonly string[]
) {
  const group = useMemo(() => new THREE.Group(), []);
  const rings = useRings(size);
  const balls = useDots(dots, size, bone, keyed);
  useEffect(() => {
    g.group.current = group;
    rings.forEach((r, i) => {
      g.rings.current[i] = r;
    });
    for (const [name, mesh] of balls) g.dotRef(name)(mesh);
  });
  const { dragging } = g;
  useEffect(() => markDragged(rings, dragging), [rings, dragging]);
  return { group, rings, balls };
}
