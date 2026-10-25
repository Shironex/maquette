import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { BENCH_TOP } from './manifest';

const LAMP_AIM: readonly [number, number, number] = [-0.02, BENCH_TOP, 0.06];

/** Refs for the lamp and the moon; once mounted, aims both and sets the lamp's shadow. */
export function useLights() {
  const lamp = useRef<THREE.SpotLight>(null);
  const moon = useRef<THREE.DirectionalLight>(null);

  useEffect(() => {
    const spot = lamp.current;
    if (spot) {
      spot.target.position.set(...LAMP_AIM);
      spot.target.updateMatrixWorld();
      spot.shadow.mapSize.set(1024, 1024);
      spot.shadow.camera.near = 0.1;
      spot.shadow.camera.far = 3;
      spot.shadow.bias = -0.0004;
      spot.shadow.normalBias = 0.012;
      spot.shadow.radius = 4;
    }
    const light = moon.current;
    if (!light) return;
    light.target.position.set(0, BENCH_TOP, 0);
    light.target.updateMatrixWorld();
  }, []);

  return { lamp, moon };
}
