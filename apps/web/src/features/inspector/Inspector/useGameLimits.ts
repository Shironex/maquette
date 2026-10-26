import type { InspectModel } from '@/ipc/client';
import { budgetFor, fileOf, gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';

/** In a game, the selected model's folder budget, which the Model panel checks it against. */
export function useGameLimits(model: InspectModel | null) {
  const project = useAppStore(s => s.workspace?.project);
  const game = gameOf(project);
  if (!game) return null;
  const asset = project?.assets?.find(a => a.id === model?.asset);
  return { triangles: asset ? (budgetFor(game, fileOf(game, asset))?.triangles ?? null) : null };
}
