/**
 * The Layout editor's 3D view: every model of the project where the layout
 * puts it, the desk, a ground grid and the camera stops. A click selects a
 * model or a stop and three's transform gizmo moves, turns or scales it;
 * a drag elsewhere orbits. Looking through a stop flies the camera there.
 * Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import { Editor } from './Editor';
import type { LayoutCanvasProps } from './LayoutCanvas.types';

export default function LayoutCanvas({ className, ...props }: LayoutCanvasProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 35, position: [1.4, 1, 1.6], near: 0.01, far: 200 }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      aria-label="Scene layout"
    >
      <Editor {...props} />
    </Canvas>
  );
}
