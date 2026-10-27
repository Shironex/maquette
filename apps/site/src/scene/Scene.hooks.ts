import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef } from 'react';
import { FrameTimes, adaptiveDpr } from './quality';
import type { TourScroll } from './tour';

/**
 * How fast the scene's progress chases the page's, per second. Light: it
 * smooths the steps of a mouse wheel but stays tied to the scroll.
 */
const FOLLOW = 6;
/** The longest frame the easing trusts, so a stalled tab does not jump. */
const MAX_STEP = 0.05;
/** Progress left, in stops, under which it lands. */
const SETTLED = 1e-4;

/**
 * Tour progress as a ref the frame loops read: eased a little behind the
 * page's (exactly on it when `still`, for poster capture). Runs before the
 * other frame loops so they all read the same value.
 */
export function useProgress(scroll: TourScroll, still: boolean) {
  const goal = useRef(scroll.read());
  const ref = useRef(goal.current);
  const invalidate = useThree(state => state.invalidate);
  useEffect(
    () =>
      scroll.subscribe(progress => {
        goal.current = progress;
        invalidate();
      }),
    [scroll, invalidate]
  );
  useFrame((_, delta) => {
    const left = goal.current - ref.current;
    if (still || Math.abs(left) < SETTLED) {
      ref.current = goal.current;
      return;
    }
    ref.current += left * (1 - Math.exp(-FOLLOW * Math.min(delta, MAX_STEP)));
    invalidate();
  }, -1);
  return ref;
}

/**
 * Keeps frames coming while the scene lives (the camera breathes, the
 * turntable idles, dust drifts), but only while the canvas is on screen and
 * the tab is shown. Poster capture draws on demand only.
 */
export function useLiveFrames(still: boolean) {
  const { gl, invalidate } = useThree();
  const shown = useRef(true);
  useEffect(() => {
    if (still) return;
    const wake = () => {
      if (shown.current && !document.hidden) invalidate();
    };
    const observer = new IntersectionObserver(([entry]) => {
      shown.current = entry?.isIntersecting ?? true;
      wake();
    });
    observer.observe(gl.domElement);
    document.addEventListener('visibilitychange', wake);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', wake);
    };
  }, [gl, invalidate, still]);
  useFrame(() => {
    if (!still && shown.current && !document.hidden) invalidate();
  });
}

/**
 * Draws at the ratio `adaptiveDpr` picks: the Canvas range as the ceiling,
 * capped by the pixel budget, and dropped by R3F's performance factor while
 * the camera moves on a GPU whose frames run slow. Only frames drawn at full
 * quality are timed, so a drop lasts while the scroll does and then lifts.
 * Poster capture keeps the cap but never drops.
 */
export function useAdaptiveDpr(scroll: TourScroll, still: boolean) {
  const times = useRef(new FrameTimes());
  const regress = useThree(state => state.performance.regress);
  useEffect(() => {
    if (still) return;
    return scroll.subscribe(() => {
      if (times.current.slow()) regress();
    });
  }, [scroll, still, regress]);
  useFrame(state => {
    const { performance: quality, size, viewport, setDpr } = state;
    const full = quality.current >= quality.max;
    times.current.tick(performance.now(), !still && full);
    const dpr = adaptiveDpr(window.devicePixelRatio, size, still ? 1 : quality.current);
    // R3F sets the range's ratio again when the Canvas configures or the display changes.
    if (dpr !== viewport.dpr) setDpr(dpr);
  });
}
