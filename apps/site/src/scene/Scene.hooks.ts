import { useThree } from '@react-three/fiber';
import { useEffect, useRef } from 'react';
import type { StopId } from './manifest';
import type { SceneProps } from './Workbench.types';

/** The current stop as a ref the frame loops read; a stop change requests a frame. */
export function useStop(initial: StopId, subscribe: SceneProps['subscribe']) {
  const ref = useRef(initial);
  const invalidate = useThree(state => state.invalidate);
  useEffect(
    () =>
      subscribe(stop => {
        ref.current = stop;
        invalidate();
      }),
    [subscribe, invalidate]
  );
  return ref;
}
