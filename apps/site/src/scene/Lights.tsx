/**
 * Night on the bench: the desk lamp is the warm key and the only shadow
 * caster, moonlight comes cool through the window behind, and a low warm
 * bounce keeps the front of the props from going black.
 */
import { BENCH_TOP } from './manifest';
import { useLights } from './Lights.hooks';

/** Inside the lamp's shade, which hangs over the right of the bench. */
export const LAMP_BULB: readonly [number, number, number] = [0.6, BENCH_TOP + 0.44, -0.2];

export function Lights() {
  const { lamp, moon } = useLights();

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
