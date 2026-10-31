import { useEffect } from 'react';
import { preloadWhenIdle } from './preloadWhenIdle';

/** Fetches the views' code while the app is quiet, until the shell goes. */
export function usePreloadViews(loads: readonly (() => Promise<unknown>)[]) {
  useEffect(() => preloadWhenIdle(loads), [loads]);
}
