import { useEffect, useEffectEvent } from 'react';
import type { LoadedMotion } from '../../MotionStage/Scene/Scene.types';
import { useMotionModel } from '../../MotionStage/Scene/Scene.hooks';

/** Loads the skeleton at `url` and tells `onLoad` when it is in, and null once it goes. */
export function useSkeletonLoad(
  url: string,
  onLoad: (url: string, loaded: LoadedMotion | null) => void,
  onError?: (message: string) => void
) {
  const loaded = useMotionModel(url, onError);
  const tell = useEffectEvent((next: LoadedMotion | null) => onLoad(url, next));
  useEffect(() => {
    tell(loaded);
    return () => tell(null);
  }, [loaded]);
}
