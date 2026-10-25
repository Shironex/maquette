import { useEffect, useMemo } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import type * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/**
 * Orbit and zoom around `target`, damped, spinning slowly while `turntable`
 * is on until someone drags.
 */
export function useOrbitControls(turntable: boolean, target: THREE.Vector3 | null): OrbitControls {
  const { camera, gl } = useThree();
  const controls = useMemo(() => new OrbitControls(camera, gl.domElement), [camera, gl]);
  useEffect(() => {
    if (target) controls.target.copy(target);
  }, [controls, target]);
  useEffect(() => {
    controls.enableDamping = true;
    controls.autoRotate = turntable;
    controls.autoRotateSpeed = 0.8;
    const stop = () => {
      controls.autoRotate = false;
    };
    controls.addEventListener('start', stop);
    return () => {
      controls.removeEventListener('start', stop);
      controls.dispose();
    };
  }, [controls, turntable]);
  useFrame(() => controls.update());
  return controls;
}
