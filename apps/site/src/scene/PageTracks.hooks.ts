import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useMemo } from 'react';
import { STYLE_TRACK, TEXT_TRACK, TIMELINE } from './manifest';
import { styleAt, textAt } from './page-tracks';

/**
 * Writes the look on the canvas and the opacity on the copy the site marks
 * with `data-maquette-text`: only when a value moves, and nothing at all
 * without a track.
 */
export function usePageTracks(progress: React.RefObject<number>) {
  const canvas = useThree(s => s.gl.domElement);
  const copy = useMemo(
    () =>
      TEXT_TRACK && typeof document !== 'undefined'
        ? [...document.querySelectorAll<HTMLElement>('[data-maquette-text]')]
        : [],
    []
  );
  const shown = useMemo(() => ({ filter: 'none', opacity: 1 }), []);
  useEffect(
    () => () => {
      canvas.style.filter = '';
      for (const el of copy) el.style.opacity = '';
    },
    [canvas, copy]
  );
  useFrame(() => {
    if (!STYLE_TRACK && !TEXT_TRACK) return;
    const filter = styleAt(STYLE_TRACK, TIMELINE.stopIds, progress.current);
    if (filter !== shown.filter) {
      shown.filter = filter;
      canvas.style.filter = filter === 'none' ? '' : filter;
    }
    const opacity = textAt(TEXT_TRACK, TIMELINE.stopIds, progress.current);
    if (opacity === shown.opacity) return;
    shown.opacity = opacity;
    for (const el of copy) el.style.opacity = String(opacity);
  });
}
