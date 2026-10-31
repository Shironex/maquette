/** Long enough after the start that the first view has settled. */
const FIRST_MS = 1500;
/** The web views the desktop app runs on do not all have `requestIdleCallback`. */
const NEXT_MS = 150;

function whenQuiet(run: () => void, ms: number): () => void {
  const timer = setTimeout(() => {
    if ('requestIdleCallback' in window) window.requestIdleCallback(run, { timeout: 1000 });
    else run();
  }, ms);
  return () => clearTimeout(timer);
}

/**
 * Fetches view code one module at a time while the app is quiet, so a first
 * click rarely waits. A chunk that fails stays for the click to report.
 * Returns the way to stop. Tests load only what they open.
 */
export function preloadWhenIdle(loads: readonly (() => Promise<unknown>)[]): () => void {
  if (import.meta.env.MODE === 'test') return () => undefined;
  let stopped = false;
  let cancel = () => undefined as void;
  const next = (at: number) => {
    const load = loads[at];
    if (stopped || !load) return;
    cancel = whenQuiet(
      () => {
        if (stopped) return;
        void load()
          .catch(() => undefined)
          .then(() => next(at + 1));
      },
      at === 0 ? FIRST_MS : NEXT_MS
    );
  };
  next(0);
  return () => {
    stopped = true;
    cancel();
  };
}
