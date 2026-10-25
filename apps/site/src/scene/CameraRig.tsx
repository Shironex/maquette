/**
 * Flies the camera through the bench as the page scrolls: each stop's move
 * plays while its copy is on screen and the path sweeps on to the next stop
 * between them, with a slow breath at rest and a lean after the pointer.
 * Stops are framed for 16:10: on a narrower window the field of view widens
 * so the subject stays in frame.
 */
import { useCameraRig, type CameraRigOptions } from './CameraRig.hooks';

export type CameraRigProps = CameraRigOptions;

export function CameraRig(props: CameraRigProps) {
  useCameraRig(props);
  return null;
}
