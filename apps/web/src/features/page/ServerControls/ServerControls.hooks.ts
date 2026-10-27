import { serverOwner } from '@/state/page';
import { usePageStore } from '@/state/use-page-store';

/** Who keeps the open project's dev server going, and the project Stop acts on. */
export function useServerControls() {
  const view = usePageStore(s => s.view);
  const server = view?.server ?? null;
  const live = server?.state === 'running' || server?.state === 'starting';
  return {
    project: view?.project ?? null,
    // The Page view starting its own server is the usual case; say so only otherwise.
    owner:
      live && server && (server.owner !== 'page' || server.external) ? serverOwner(server) : null,
  };
}
