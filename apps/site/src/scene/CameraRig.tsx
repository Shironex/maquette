/**
 * Eases the camera to the current stop. Damped, so a scroll step settles in
 * about 1.2 s; with reduced motion it cuts. Stops are framed for 16:10: on a
 * narrower window the field of view widens so the subject stays in frame.
 */
import { useCameraRig, type CameraRigOptions } from './CameraRig.hooks';

export type CameraRigProps = CameraRigOptions;

export function CameraRig(props: CameraRigProps) {
  useCameraRig(props);
  return null;
}
