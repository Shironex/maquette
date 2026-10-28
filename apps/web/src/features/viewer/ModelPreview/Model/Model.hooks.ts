import { useEffect, useMemo } from 'react';
import { applyMode, checkerTexture } from '../../materials';
import type { PartLook } from '../../parts';
import type { Loaded, PreviewMode } from '../ModelPreview.types';

/** Dresses the model in `mode`'s materials, and back in its own on a change. */
export function useModelLook(loaded: Loaded, mode: PreviewMode, parts?: PartLook): void {
  const checker = useMemo(() => checkerTexture(), []);
  const key = JSON.stringify(parts ?? null);
  useEffect(() => {
    const look = key === 'null' ? undefined : (JSON.parse(key) as PartLook);
    return applyMode(loaded.originals, mode, checker, look).undo;
  }, [loaded, mode, checker, key]);
}
