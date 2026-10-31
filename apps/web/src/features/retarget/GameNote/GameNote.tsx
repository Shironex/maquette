import { formatBytes, formatK } from '@/lib/format';
import { budgetFor, fileOf, gameOf } from '@/state/game';
import { budgetLine } from '@/state/motion';
import { useGameNote } from './GameNote.hooks';

export interface GameNoteProps {
  asset: string | null;
  name: string;
  /** What the chosen clips add, bytes. */
  adds: number;
}

/**
 * What the chosen clips mean for the budget: a game's folder budget counts
 * triangles and texture size, so clips never push a model over it; a website
 * weighs them in the scene budget.
 */
export default function GameNote({ asset, name, adds }: GameNoteProps) {
  const { project, view, draft } = useGameNote();
  const game = gameOf(project);
  if (game) {
    const found = project?.assets?.find(a => a.id === asset);
    const budget = found ? budgetFor(game, fileOf(game, found)) : null;
    if (!budget) return null;
    return (
      <p className="text-meta text-fg-mute">
        Game budget for <span className="font-mono">{budget.folder}/</span> counts triangles (
        {formatK(budget.triangles)}) and texture size ({budget.texture} px), so clips never push{' '}
        {name} over it.
      </p>
    );
  }
  if (!view) return null;
  const line = budgetLine(view, draft);
  const after = line.sceneMb + adds / 1e6;
  return (
    <p className="text-meta text-fg-mute">
      Scene budget: clips are {formatBytes(line.motion)} now, about{' '}
      {formatBytes(line.motion + adds)} with these. The scene goes to {after.toFixed(2)} of{' '}
      {line.targetMb} MB
      {after > line.targetMb ? ', over its budget' : ''}.
    </p>
  );
}
