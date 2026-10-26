/**
 * The Motion view's 3D stage: one model in the preview studio over a floor
 * grid, playing a clip at the time the player says, with its skeleton, the
 * path its root walks, or its parts posed for an object state. Orbit and
 * zoom as in the preview. Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import { Scene } from './Scene';
import type { MotionStageProps } from './MotionStage.types';

export default function MotionStage({ className, label, ...props }: MotionStageProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [2, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      aria-label={label}
    >
      <Scene {...props} />
    </Canvas>
  );
}
