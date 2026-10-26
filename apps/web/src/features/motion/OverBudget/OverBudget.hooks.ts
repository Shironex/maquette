import { formatBytes } from '@/lib/format';
import { budgetLine, clipsToFit } from '@/state/motion';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';

/** The most ways to fit the board lists, heaviest clip first. */
const WAYS = 3;

/** Why the scene is over its budget with the clips in it, and the ways to fit. */
export function useOverBudget() {
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  const setShip = useMotionStore(s => s.setShip);
  const supported = useMotionStore(s => s.view?.clipsOffSupported ?? false);
  const setView = useAppStore(s => s.setView);
  if (!view) return null;
  const line = budgetLine(view, draft);
  if (!line.over) return null;
  const fit = clipsToFit(view, draft);
  const title = `The scene is ${line.sceneMb.toFixed(2)} MB, over its ${line.targetMb} MB budget by ${formatBytes(line.overBy)}`;
  const detail = `Clips are ${formatBytes(line.motion)} of it. Motion counts inside the scene budget${line.firstViewOver ? ', and the first camera stop waits for the clips its models carry' : ''}.`;
  return {
    title,
    detail,
    ways: fit.slice(0, WAYS).map(({ model, clip, bytes, fits }) => ({
      id: `${model.asset}/${clip.name}`,
      title: `Keep ${clip.name} off the site`,
      detail: `${model.name}. Stays in the model for later, saves ${formatBytes(bytes)}${fits ? ', and then the scene fits' : ''}`,
      apply: () => setShip(model.asset, clip.name, false),
    })),
    supported,
    openPipeline: () => setView('pipeline'),
  };
}
