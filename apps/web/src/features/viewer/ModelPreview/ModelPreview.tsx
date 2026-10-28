/**
 * A 3D preview of one GLB or glTF file in the studio (see `studio.ts`):
 * shaded, a wireframe over flat clay, a UV checker, or the skeleton drawn in
 * clay over a ghost of the mesh, with orbit and zoom. Reads meshopt geometry
 * and KTX2 textures. The Inbox uses it; the Inspector shares its studio and
 * materials for the compare view.
 * Loaded lazily, so three.js stays out of the first bundle.
 */
import { Canvas } from '@react-three/fiber';
import { BACKDROP } from '../studio';
import { Studio } from './Studio';
import type { ModelPreviewProps } from './ModelPreview.types';

export default function ModelPreview({
  mode = 'shaded',
  turntable = true,
  className,
  parts,
  onPickPart,
  ...source
}: ModelPreviewProps) {
  return (
    <Canvas
      className={className}
      shadows
      camera={{ fov: 30, position: [2, 1.4, 3] }}
      dpr={[1, 2]}
      gl={{ antialias: true, preserveDrawingBuffer: true }}
      role="img"
      aria-label="Model preview"
      onPointerMissed={onPickPart ? () => onPickPart(null, false) : undefined}
    >
      <color attach="background" args={[BACKDROP]} />
      <Studio
        source={source}
        mode={mode}
        turntable={turntable}
        parts={parts}
        onPickPart={onPickPart}
      />
    </Canvas>
  );
}
