/**
 * The Avatar view's Preview stage: a written `.vrm` loaded the way a VRM
 * app loads it (three-vrm on the viewer's glTF loader), in the preview
 * studio, with its spring bones and look at running every frame. Loaded
 * lazily, so three-vrm ships in its own chunk.
 */
import { Canvas } from '@react-three/fiber';
import { Scene } from './Scene';
import type { VrmStageProps } from './VrmStage.types';

export default function VrmStage({ className, label, ...props }: VrmStageProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [0, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      role="img"
      aria-label={label}
    >
      <Scene {...props} />
    </Canvas>
  );
}
