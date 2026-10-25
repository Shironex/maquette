/**
 * Moves the camera along the tour as the page scrolls: each stop plays its
 * move and the path runs on into the next; with reduced motion it cuts from
 * stop to stop. Stops are framed for 16:10: on a narrower window the field
 * of view widens so the subject stays in frame.
 */
import { useCameraRig, type CameraRigOptions } from './CameraRig.hooks';

export type CameraRigProps = CameraRigOptions;

export function CameraRig(props: CameraRigProps) {
  useCameraRig(props);
  return null;
}
