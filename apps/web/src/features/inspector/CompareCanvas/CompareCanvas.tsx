/**
 * Two versions of a scene or an asset in one canvas, split down the middle:
 * each side is its own three.js scene, both drawn with the same camera, the
 * left one scissored to the left of the split and the right one to the
 * rest, so orbiting moves both at once. Reuses the preview studio (lights,
 * floor, framing) and its material modes.
 *
 * It also measures itself: every fourth frame, and the first of each
 * sample, waits for the GPU to finish,
 * so its time covers updating and drawing both sides on the CPU and the
 * GPU; the samples average over 250 ms, next to the frame rate.
 * Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import { Compare } from './Compare';
import type { CompareCanvasProps } from './CompareCanvas.types';

export default function CompareCanvas({ className, ...props }: CompareCanvasProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [6, 4, 10] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      aria-label="Compare view"
    >
      <Compare {...props} />
    </Canvas>
  );
}
