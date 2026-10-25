import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useRef } from 'react';
import type { TourScroll } from './tour';

/**
 * How fast the scene's progress chases the page's, per second. Light: it
 * smooths the steps of a mouse wheel but stays tied to the scroll.
 */
const FOLLOW = 7;
/** The longest frame the easing trusts, so a stalled tab does not jump. */
const MAX_STEP = 0.05;
/** Progress left, in stops, under which it lands and stops drawing. */
const SETTLED = 1e-4;

/**
 * Tour progress as a ref the frame loops read: eased a little behind the
 * page's (exactly on it with reduced motion), redrawing until it lands.
 * Runs before the other frame loops so they all read the same value.
 */
export function useProgress(scroll: TourScroll, reducedMotion: boolean) {
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
    if (reducedMotion || Math.abs(left) < SETTLED) {
      ref.current = goal.current;
      return;
    }
    ref.current += left * (1 - Math.exp(-FOLLOW * Math.min(delta, MAX_STEP)));
    invalidate();
  }, -1);
  return ref;
}
