import type * as THREE from 'three';
import { useOrbitControls } from './Controls.hooks';

export interface ControlsProps {
  turntable: boolean;
  target: THREE.Vector3 | null;
}

/** The preview's orbit controls; draws nothing. */
export default function Controls({ turntable, target }: ControlsProps) {
  useOrbitControls(turntable, target);
  return null;
}
