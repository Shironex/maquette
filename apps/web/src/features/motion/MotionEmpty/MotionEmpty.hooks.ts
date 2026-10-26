import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { isGame } from '@/state/game';
import { useMotionStore } from '@/state/use-motion-store';

/** The project's name and size, and the prop with the most parts, worth giving states. */
export function useMotionEmpty() {
  const workspace = useAppStore(s => s.workspace);
  const view = useMotionStore(s => s.view);
  const giveStates = useMotionStore(s => s.giveStates);
  const models = view?.models ?? [];
  const prop = models
    .filter(m => m.kind !== 'character' && !m.error && m.parts.length > 1)
    .sort((a, b) => b.parts.length - a.parts.length)[0];
  const thumb =
    prop?.thumbnail && workspace ? fileUrl(projectFile(workspace, prop.thumbnail)) : null;
  return {
    name: workspace?.project?.name ?? 'This project',
    count: models.length,
    game: isGame(workspace?.project),
    // States are a website's: a game gets no prop to give them to.
    suggestion:
      prop && !isGame(workspace?.project)
        ? { model: prop, thumb, give: () => giveStates(prop.asset) }
        : null,
  };
}
