import { homePath, shortPath } from '@/lib/format';
import { serverLine, shortAddress } from '@/state/page';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';

/** The two sources the view can show and the server's line for the first. */
export function usePageSource() {
  const view = usePageStore(s => s.view);
  const save = usePageStore(s => s.save);
  const pickConcept = usePageStore(s => s.pickConcept);
  const root = useAppStore(s => s.workspace?.root);
  const home = useAppStore(s => s.boot?.home);
  const source = view?.settings.source ?? 'site';
  const concept = view?.settings.concept ?? null;
  const conceptLine = concept
    ? root && concept.startsWith(root)
      ? shortPath(concept.slice(root.length).replace(/^[\\/]+/, ''), 2)
      : homePath(concept, home)
    : 'Pick an .html file in the workspace';
  const running = view?.server.state === 'running';
  return {
    source,
    siteLine: !view?.siteRepo
      ? 'No site repo yet'
      : running && view.server.url
        ? serverLine(view.server)
        : view.server.state !== 'idle'
          ? serverLine(view.server)
          : (view.command?.argv.join(' ') ?? 'No dev command'),
    live: running && Boolean(view?.server.url),
    host: view?.server.url ? shortAddress(view.server.url) : null,
    conceptLine,
    hasConcept: Boolean(concept),
    showSite: () => void save({ source: 'site' }),
    showConcept: () => (concept ? void save({ source: 'concept' }) : void pickConcept()),
    changeConcept: () => void pickConcept(),
  };
}
