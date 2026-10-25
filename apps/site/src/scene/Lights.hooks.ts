import { useFrame } from '@react-three/fiber';
import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { BENCH_TOP } from './manifest';

/** Where the lamp points, on the bench under the clay island. */
export const LAMP_AIM: readonly [number, number, number] = [-0.02, BENCH_TOP, 0.06];
/** Inside the lamp's shade, which hangs over the right of the bench. */
export const LAMP_BULB: readonly [number, number, number] = [0.6, BENCH_TOP + 0.44, -0.2];

/** The lamp's strength, and how much its filament wavers (a share of it). */
export const LAMP = { intensity: 5.5, glow: 0.5, waver: 0.03 };

/**
 * Refs for the lamp, its glow and the moon; once mounted, aims the lamp and
 * the moon and sets the lamp's shadow. Unless `still`, the filament wavers a
 * little, never in step with itself.
 */
export function useLights(still: boolean) {
  const lamp = useRef<THREE.SpotLight>(null);
  const glow = useRef<THREE.PointLight>(null);
  const moon = useRef<THREE.DirectionalLight>(null);

  useFrame(state => {
    if (still || !lamp.current || !glow.current) return;
    const t = state.clock.elapsedTime;
    const waver =
      1 +
      LAMP.waver * (0.6 * Math.sin(t * 1.7) + 0.3 * Math.sin(t * 4.3 + 1) + 0.1 * Math.sin(t * 11));
    lamp.current.intensity = LAMP.intensity * waver;
    glow.current.intensity = LAMP.glow * waver;
  });

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

  return { lamp, glow, moon };
}
