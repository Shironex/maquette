import { useEffect, useState } from 'react';
import { poseStills } from '@/features/viewer/pose-snapshots';

/**
 * Stills of the staged model at each of `frames`, drawn once after the
 * sheet shows; null while they draw, and an empty list when they cannot
 * be drawn here (no model, no WebGL, a model that does not load).
 */
export function useSheetStills(
  url: string | null,
  clip: string,
  frames: readonly number[],
  last: number
): string[] | null {
  const key = frames.join(',');
  const [stills, setStills] = useState<{ key: string; urls: string[] } | null>(null);
  useEffect(() => {
    if (!url || !key) return;
    let live = true;
    const wanted = key.split(',').map(Number);
    poseStills(url, clip, wanted, last)
      .then(urls => {
        if (live) setStills({ key: `${url}|${key}`, urls });
      })
      .catch(() => {
        if (live) setStills({ key: `${url}|${key}`, urls: [] });
      });
    return () => {
      live = false;
    };
  }, [url, clip, key, last]);
  if (!url) return [];
  return stills?.key === `${url}|${key}` ? stills.urls : null;
}
