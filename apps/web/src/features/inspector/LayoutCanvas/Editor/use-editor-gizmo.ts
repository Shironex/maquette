import { useEffect } from 'react';
import type * as THREE from 'three';
import type { Vec3 } from '@/state/inspector';
import { SNAP } from '@/state/layout';
import type { EditorProps, EditorRig } from './Editor.types';
import { deg, placementFrom } from './placement';

/**
 * Three's transform gizmo on the selection, in the tool's mode with the
 * snapping, and what its drags change: a model as it moves, a stop once it
 * is let go.
 */
export function useEditorGizmo(editor: EditorRig, props: EditorProps, span: number): void {
  const { holders, live, orbit, transform, stopsGroup, box, dragStart } = editor;

  // The gizmo follows the selection: a model's holder or a stop's camera.
  useEffect(() => {
    const sel = props.selection;
    let target: THREE.Object3D | undefined;
    if (sel?.kind === 'model') target = holders?.get(sel.id);
    if (sel?.kind === 'stop') {
      target = stopsGroup.children.find(g => g.userData.stop === sel.id);
    }
    if (target && !props.preview) transform.attach(target);
    else transform.detach();
    box.visible = Boolean(sel?.kind === 'model' && target);
  }, [
    props.selection,
    props.preview,
    holders,
    stopsGroup,
    transform,
    box,
    props.layout.stops,
    span,
  ]);

  useEffect(() => {
    const stop = props.selection?.kind === 'stop';
    transform.setMode(stop ? 'translate' : props.tool);
    transform.setSpace(props.tool === 'rotate' ? 'local' : 'world');
    transform.setTranslationSnap(props.snap ? SNAP.move : null);
    transform.setRotationSnap(props.snap ? deg(SNAP.rotate) : null);
    transform.setScaleSnap(props.snap ? SNAP.scale : null);
    transform.setSize(0.8);
  }, [transform, props.tool, props.snap, props.selection]);

  useEffect(() => {
    const onDragging = (e: { value: unknown }) => {
      orbit.enabled = !e.value;
      const object = transform.object;
      if (e.value) {
        dragStart.current = object?.scale.x ?? 1;
        live.current.handlers.onBeginDrag();
      } else if (object?.userData.stop) {
        // A stop rebuilds its glyph when it changes, so it is saved once, at the end.
        live.current.handlers.onDragStop(
          object.userData.stop as string,
          object.position.toArray() as Vec3
        );
      }
    };
    const onChange = () => {
      const object = transform.object;
      const p = live.current;
      if (!object || object.userData.stop) return;
      const asset = object.userData.asset as string | undefined;
      if (!asset) return;
      if (transform.getMode() === 'scale') {
        // Scale stays uniform: the axis pulled furthest sets all three.
        const s = object.scale;
        const was = dragStart.current;
        const next = [s.x, s.y, s.z].reduce((a, b) =>
          Math.abs(b - was) > Math.abs(a - was) ? b : a
        );
        object.scale.setScalar(Math.max(0.01, next));
      }
      p.handlers.onDragModel(asset, placementFrom(object));
    };
    transform.addEventListener('dragging-changed', onDragging);
    transform.addEventListener('objectChange', onChange);
    return () => {
      transform.removeEventListener('dragging-changed', onDragging);
      transform.removeEventListener('objectChange', onChange);
      transform.detach();
      transform.dispose();
    };
  }, [transform, orbit, dragStart, live]);
}
