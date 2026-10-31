import { useEffect, useState } from 'react';
import { commands, errorMessage, type BonePair, type MapChoice } from '@/ipc/client';

/**
 * The bone map one clip would play through on a model, as the plan reads
 * it. Read only, and read again when the clip, the model or the map changes.
 */
export function useRoughMap(
  project: string | null,
  asset: string | null,
  clip: string | null,
  picked: readonly MapChoice[]
) {
  const [map, setMap] = useState<BonePair[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const picks = JSON.stringify(picked);
  useEffect(() => {
    setMap(null);
    setError(null);
    if (!project || !asset || !clip) return;
    let alive = true;
    commands
      .animationsRetargetPlan(project, asset, [clip], JSON.parse(picks) as MapChoice[])
      .then(plan => {
        if (alive) setMap(plan.map);
      })
      .catch((err: unknown) => {
        if (alive) setError(errorMessage(err));
      });
    return () => {
      alive = false;
    };
  }, [project, asset, clip, picks]);
  return { map, problem: error ?? failed, setFailed };
}
