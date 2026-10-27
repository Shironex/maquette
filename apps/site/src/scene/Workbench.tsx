/**
 * The live workbench: the manifest's models under a warm lamp and cool
 * moonlight, a camera that follows the scroll along the tour, the turntable
 * spin and the wireframe sweep. Drawn on demand: frames only render while
 * something moves.
 */
import { Canvas } from '@react-three/fiber';
import * as THREE from 'three';
import { STOPS } from './manifest';
import { DPR_RANGE, PERFORMANCE } from './quality';
import { Scene } from './Scene';
import { GPU_CONTEXT } from './tour';
import type { WorkbenchProps } from './Workbench.types';

export type { WorkbenchProps } from './Workbench.types';

export function Workbench(props: WorkbenchProps) {
  const start = STOPS.find(s => s.id === props.initialStop) ?? STOPS[0];
  return (
    <Canvas
      frameloop="demand"
      dpr={DPR_RANGE}
      performance={PERFORMANCE}
      shadows="percentage"
      camera={{ fov: start?.fov ?? 36, near: 0.05, far: 30, position: start?.position }}
      gl={{ antialias: true, powerPreference: 'high-performance', stencil: false, ...GPU_CONTEXT }}
      onCreated={({ gl }) => {
        if (props.capture) Object.assign(window, { __maquetteRenderer: gl });
        gl.toneMapping = THREE.ACESFilmicToneMapping;
        gl.toneMappingExposure = 1.15;
        gl.domElement.addEventListener('webglcontextlost', () => props.onFailed('context lost'));
      }}
      aria-hidden="true"
      tabIndex={-1}
    >
      <Scene {...props} />
    </Canvas>
  );
}
