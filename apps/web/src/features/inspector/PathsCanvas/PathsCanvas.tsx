/**
 * The Paths editor's 3D view: the models where the layout puts them and
 * every path as a tube, the selected one with a handle on each point. A
 * click picks a point or a path, the transform gizmo moves the picked
 * point, and Add points puts a point where a click lands on a model or the
 * ground. A drag elsewhere orbits. Loaded lazily, with three.js.
 */
import { Canvas } from '@react-three/fiber';
import { PathsScene } from './PathsScene';
import type { PathsCanvasProps } from './PathsCanvas.types';

export default function PathsCanvas({ className, ...props }: PathsCanvasProps) {
  return (
    <Canvas
      className={className}
      camera={{ fov: 35, position: [6, 5, 9], near: 0.01, far: 400 }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      role="img"
      aria-label="Paths in the scene"
    >
      <PathsScene {...props} />
    </Canvas>
  );
}
