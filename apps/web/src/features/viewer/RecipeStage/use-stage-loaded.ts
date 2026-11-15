import { useEffect, useEffectEvent } from 'react';

/** Tells `onLoaded` once the stage is set up, so its first frame is on screen. */
export function useStageLoaded(setup: unknown, onLoaded: (() => void) | undefined) {
  const loaded = useEffectEvent(() => onLoaded?.());
  useEffect(() => {
    if (setup) loaded();
  }, [setup]);
}
