import type { LoadedMotion } from '../../MotionStage/Scene/Scene.types';
import { useSkeletonLoad } from './SkeletonLoad.hooks';

export interface SkeletonLoadProps {
  url: string;
  onLoad: (url: string, loaded: LoadedMotion | null) => void;
  onError?: (message: string) => void;
}

/** Loads one more clip's skeleton for a joined recipe, inside the canvas; it draws nothing. */
export default function SkeletonLoad({ url, onLoad, onError }: SkeletonLoadProps) {
  useSkeletonLoad(url, onLoad, onError);
  return null;
}
