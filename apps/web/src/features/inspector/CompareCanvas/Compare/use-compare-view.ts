import { useEffect, useEffectEvent } from 'react';
import type * as THREE from 'three';
import { applyMode } from '@/features/viewer/materials';
import { explode } from '@/features/viewer/parts';
import { frameCamera, standOnFloor, studioRig, type Framing } from '@/features/viewer/studio';
import { flightStart } from '@/lib/motion';
import { FLIGHT_MS } from '@/state/inspector';
import { poseOf, setPose } from '../../camera-pose';
import type { CompareProps, CompareRig, Side } from './Compare.types';
import { applyLooks } from './apply-looks';
import { disposeGroup } from './dispose-group';
import { sceneFraming } from './scene-framing';

/**
 * Both sides in their scenes with the studio's light, the first view, the
 * flights to stops, the orbit, the material modes and the tweak preview.
 */
export function useCompareView(rig: CompareRig, props: CompareProps): void {
  const { camera, left, right, scenes, controls, checker, framing, setFraming } = rig;
  const { live, flightRef, shotsPending } = rig;

  // Both sides in place: frame on the current (right) side and light both.
  useEffect(() => {
    const main = right ?? left;
    if (!main) return;
    let next: Framing | null;
    if (props.kind === 'asset') {
      if (left && left !== main) standOnFloor(left.group);
      next = standOnFloor(main.group);
    } else {
      next = sceneFraming(main.group);
    }
    if (!next) return;
    const rigs: THREE.Group[] = [];
    for (const [scene, side] of [
      [scenes.left, left],
      [scenes.right, right],
    ] as const) {
      scene.clear();
      if (!side) continue;
      const studio = studioRig(next);
      if (props.kind === 'scene') studio.position.y = next.box.min.y;
      rigs.push(studio);
      scene.add(studio, side.group);
    }
    camera.near = next.extent / 400;
    camera.far = next.extent * 60;
    camera.updateProjectionMatrix();
    setFraming(next);
    live.current.handlers.onReady();
    return () => {
      for (const studio of rigs) {
        studio.removeFromParent();
        disposeGroup(studio);
      }
    };
  }, [left, right, props.kind, scenes, camera, setFraming, live]);

  // The first view: the current stop, or the studio's three quarter framing.
  useEffect(() => {
    if (!framing) return;
    const p = live.current;
    const stop = p.kind === 'scene' && p.stop ? p.stops[p.stop.index] : undefined;
    if (stop) {
      setPose(camera, controls, { position: stop.position, target: stop.target, fov: stop.fov });
    } else {
      camera.fov = 30;
      frameCamera(camera, framing);
      controls.target.copy(framing.target);
    }
    shotsPending.current = p.kind === 'scene' && p.stops.some(s => !s.poster);
  }, [framing, camera, controls, live, shotsPending]);

  // A stop was clicked: fly there. Only a new click starts a flight; the
  // stops list changing does not.
  const flyToStop = useEffectEvent(() => {
    if (!framing || !props.stop || props.kind !== 'scene') return;
    const stop = props.stops[props.stop.index];
    if (!stop) return;
    flightRef.current = {
      from: poseOf(camera, controls.target),
      stop,
      start: flightStart(FLIGHT_MS),
    };
    controls.autoRotate = false;
  });
  useEffect(() => flyToStop(), [props.stop?.at, framing]);

  useEffect(() => {
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.autoRotateSpeed = 0.6;
    const start = () => {
      flightRef.current = null;
      controls.autoRotate = false;
      live.current.handlers.onUserMove();
    };
    controls.addEventListener('start', start);
    return () => {
      controls.removeEventListener('start', start);
      controls.dispose();
    };
  }, [controls, flightRef, live]);

  useEffect(() => {
    controls.autoRotate = props.turntable;
  }, [controls, props.turntable]);

  // Material modes on both sides.
  useEffect(() => {
    const applied = [left, right]
      .filter((s): s is Side => Boolean(s))
      .map(s => applyMode(s.originals, props.mode, checker));
    if (props.mode === 'lightmap') {
      live.current.handlers.onLightmaps(applied.reduce((n, a) => n + a.lightmapped, 0));
    }
    return () => applied.forEach(a => a.undo());
  }, [left, right, props.mode, checker, live]);

  // Parts pulled apart from each side's middle, to see where each one ends.
  useEffect(() => {
    const amount = props.explode ?? 0;
    const undo = [left, right]
      .filter((s): s is Side => Boolean(s))
      .map(s => explode(s.group, amount));
    return () => undo.forEach(u => u());
  }, [left, right, props.explode]);

  // The material tweak preview, over the shaded look only. It runs after
  // the modes, so it starts from the loaded materials they put back.
  useEffect(() => {
    const looks = props.looks;
    if (props.mode !== 'shaded' || !looks) return;
    const undo = [left, right].filter((s): s is Side => Boolean(s)).map(s => applyLooks(s, looks));
    return () => undo.forEach(u => u());
  }, [left, right, props.mode, props.looks]);
}
