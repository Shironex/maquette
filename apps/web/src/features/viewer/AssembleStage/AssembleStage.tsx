/**
 * The parts of a prop to assemble, each in a holder where its pivot
 * stands, with three's transform gizmo on the selected one: a drag moves
 * it, or turns it about its pivot, and reports the same place and turn the
 * Assemble sheet takes by numbers. A click selects a part; a drag
 * elsewhere orbits. Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import { Placer } from './Placer';
import type { AssembleStageProps } from './AssembleStage.types';

export default function AssembleStage({ className, ...props }: AssembleStageProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 35, position: [1.6, 1.2, 2.2], near: 0.01, far: 200 }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      role="img"
      aria-label="Assembly"
    >
      <Placer {...props} />
    </Canvas>
  );
}
