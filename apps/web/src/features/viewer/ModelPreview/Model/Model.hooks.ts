import { useEffect, useMemo } from 'react';
import { applyMode, checkerTexture } from '../../materials';
import type { Loaded, PreviewMode } from '../ModelPreview.types';

/** Dresses the model in `mode`'s materials, and back in its own on a change. */
export function useModelLook(loaded: Loaded, mode: PreviewMode): void {
  const checker = useMemo(() => checkerTexture(), []);
  useEffect(() => applyMode(loaded.originals, mode, checker).undo, [loaded, mode, checker]);
}
