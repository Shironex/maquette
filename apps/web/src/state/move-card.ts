/**
 * Moving cards between columns. A card moves at once, and each project's moves
 * reach the core one at a time, so they land in the order they were made. The
 * Board shows what the core answers only once the last move has settled: an
 * earlier answer would undo a later move still on its way. After a failure each
 * card goes back to the last stage the core confirmed for it.
 */
import { commands, errorMessage, type Asset, type ProjectManifest, type Stage } from '@/ipc/client';
import { withStage } from './board';
import { useAppStore } from './use-app-store';

interface Moves {
  last: Promise<void>;
  left: number;
  /** The stage on disk of each card moved since the queue was last empty. */
  confirmed: Map<string, Stage>;
  /** The project as the newest move left it; null when the newest move failed. */
  answer: ProjectManifest | null;
}

/** Outside any component: a remount must not forget a move on its way. */
const queues = new Map<string, Moves>();

function manifest(project: string): ProjectManifest | undefined {
  const workspace = useAppStore.getState().workspace;
  return workspace?.project?.id === project
    ? workspace.project
    : workspace?.projects.find(p => p.id === project);
}

function land(project: string, moves: Moves) {
  const app = useAppStore.getState();
  if (moves.answer) return app.setProject(moves.answer);
  const shown = manifest(project);
  if (!shown) return;
  app.setProject(
    [...moves.confirmed].reduce((next, [id, stage]) => withStage(next, id, stage), shown)
  );
}

async function send(project: string, moves: Moves, asset: Pick<Asset, 'id' | 'name'>, to: Stage) {
  try {
    moves.answer = await commands.assetMove(project, asset.id, to);
    moves.confirmed.set(asset.id, to);
  } catch (error) {
    moves.answer = null;
    useAppStore.getState().notify({
      status: 'failed',
      title: `Could not move ${asset.name}`,
      detail: errorMessage(error),
    });
  }
  moves.left -= 1;
  if (moves.left > 0) return;
  queues.delete(project);
  land(project, moves);
}

/** Moves a card now and asks the core to follow; it settles when the core has answered this move. */
export function moveCard(project: string, asset: Pick<Asset, 'id' | 'name'>, to: Stage) {
  const shown = manifest(project);
  const from = shown?.assets?.find(a => a.id === asset.id)?.stage;
  if (!shown || !from || from === to) return Promise.resolve();
  const moves = queues.get(project) ?? {
    last: Promise.resolve(),
    left: 0,
    confirmed: new Map<string, Stage>(),
    answer: null,
  };
  queues.set(project, moves);
  if (!moves.confirmed.has(asset.id)) moves.confirmed.set(asset.id, from);
  moves.left += 1;
  useAppStore.getState().setProject(withStage(shown, asset.id, to));
  moves.last =
    moves.left === 1
      ? send(project, moves, asset, to)
      : moves.last.then(() => send(project, moves, asset, to));
  return moves.last;
}
