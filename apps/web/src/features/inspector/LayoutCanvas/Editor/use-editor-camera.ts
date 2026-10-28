import { useEffect, useEffectEvent, useRef } from 'react';
import * as THREE from 'three';
import { frameCamera } from '@/features/viewer/studio';
import { flightStart } from '@/lib/motion';
import { FLIGHT_MS } from '@/state/inspector';
import { poseOf, setPose } from '../../camera-pose';
import type { EditorProps, EditorRig } from './Editor.types';
import { fitFov } from './fit-fov';

/**
 * Where the camera stands: orbiting by hand, framing every model on load and
 * when asked, and flying to and following the stop it looks through.
 */
export function useEditorCamera(editor: EditorRig, props: EditorProps): void {
  const { camera, size, holders, live, orbit, world, flightRef, fitted, report } = editor;
  const framed = useRef(false);

  useEffect(() => {
    orbit.enableDamping = true;
    orbit.dampingFactor = 0.1;
    orbit.screenSpacePanning = true;
    const start = () => {
      flightRef.current = null;
      live.current.handlers.onUserMove();
    };
    const end = () => report();
    orbit.addEventListener('start', start);
    orbit.addEventListener('end', end);
    return () => {
      orbit.removeEventListener('start', start);
      orbit.removeEventListener('end', end);
      orbit.dispose();
    };
  }, [orbit, camera, report, flightRef, live]);

  // Frame every model: on first load and when asked. Only loading and the
  // frame counter frame again, not every edit.
  const frameAll = useEffectEvent(() => {
    if (!holders || holders.size === 0) return;
    if (framed.current && props.frame === 0) return;
    framed.current = true;
    world.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(world);
    if (props.layout.desk) {
      const [w, d] = props.layout.desk;
      bounds.union(
        new THREE.Box3(new THREE.Vector3(-w / 2, 0, -d / 2), new THREE.Vector3(w / 2, 0, d / 2))
      );
    }
    if (bounds.isEmpty()) return;
    camera.fov = 35;
    frameCamera(camera, {
      box: bounds,
      extent: bounds.getSize(new THREE.Vector3()).length() || 1,
      target: bounds.getCenter(new THREE.Vector3()),
    });
    camera.near = 0.01;
    camera.far = 200;
    camera.updateProjectionMatrix();
    orbit.target.copy(bounds.getCenter(new THREE.Vector3()));
    flightRef.current = null;
    live.current.handlers.onPose(poseOf(camera, orbit.target));
  });
  useEffect(() => frameAll(), [holders, props.frame]);

  // Looking through a stop: fly there. Only a new click starts a flight.
  const flyToPreview = useEffectEvent(() => {
    if (!props.preview) return;
    const stop = live.current.layout.stops.find(s => s.id === props.preview?.id);
    if (!stop) return;
    const shown = fitFov(stop.fov, camera.aspect);
    fitted.current = { shown, real: stop.fov };
    flightRef.current = {
      from: poseOf(camera, orbit.target),
      to: { position: stop.position, target: stop.target, fov: shown },
      start: flightStart(FLIGHT_MS),
    };
  });
  useEffect(() => flyToPreview(), [props.preview?.at]);

  // While looking through a stop, edits to it show at once.
  const previewStop = props.preview
    ? props.layout.stops.find(s => s.id === props.preview?.id)
    : undefined;
  useEffect(() => {
    if (!previewStop || flightRef.current) return;
    const shown = fitFov(previewStop.fov, camera.aspect);
    fitted.current = { shown, real: previewStop.fov };
    setPose(camera, orbit, {
      position: previewStop.position,
      target: previewStop.target,
      fov: shown,
    });
  }, [previewStop, camera, orbit, size.width, size.height, flightRef, fitted]);
}
