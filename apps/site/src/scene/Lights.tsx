/**
 * Night on the bench: the desk lamp is the warm key and the only shadow
 * caster, moonlight comes cool through the window behind, and a low warm
 * bounce keeps the front of the props from going black.
 */
import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { BENCH_TOP } from './manifest';

/** Inside the lamp's shade, which hangs over the right of the bench. */
export const LAMP_BULB: readonly [number, number, number] = [0.6, BENCH_TOP + 0.44, -0.2];
const LAMP_AIM: readonly [number, number, number] = [-0.02, BENCH_TOP, 0.06];

export function Lights() {
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
    if (light) {
      light.target.position.set(0, BENCH_TOP, 0);
      light.target.updateMatrixWorld();
    }
  }, []);

  return (
    <>
      <hemisphereLight args={['#33405a', '#2a1c12', 0.75]} />
      <spotLight
        ref={lamp}
        position={LAMP_BULB}
        color="#ffb36b"
        intensity={5.5}
        distance={0}
        decay={2}
        angle={0.95}
        penumbra={0.75}
        castShadow
      />
      {/* The glow around the bulb itself, so the shade and the wall catch it. */}
      <pointLight position={LAMP_BULB} color="#ffae63" intensity={0.5} distance={1.6} decay={2} />
      <directionalLight ref={moon} position={[1.3, 2.6, -1.6]} color="#8fa3d6" intensity={0.9} />
      <pointLight
        position={[-0.9, 1.4, 1.2]}
        color="#c98a62"
        intensity={0.6}
        distance={4}
        decay={2}
      />
    </>
  );
}
