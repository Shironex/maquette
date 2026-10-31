import { useState } from 'react';

/**
 * Whether the stage has shown the model at `url` yet, and what it calls when it
 * has. The stage's canvas is up long before the model is, so its Suspense
 * fallback alone leaves a dark box.
 */
export function useStageReady(url: string | null) {
  const [shown, setShown] = useState<string | null>(null);
  return { ready: shown === url, onLoaded: () => setShown(url) };
}
