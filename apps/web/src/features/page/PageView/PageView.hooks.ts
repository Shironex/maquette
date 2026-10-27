import { useEffect } from 'react';
import { frameSource, frameState, siteName, type FrameState } from '@/state/page';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';

/** How often the view reads the server again while it starts, beside its events. */
const STARTING_POLL = 1500;

/** Opens the page for the project and lets go of its server when the view or the project changes. */
export function usePageLifecycle(projectId: string | null) {
  const open = usePageStore(s => s.open);
  const close = usePageStore(s => s.close);
  const refresh = usePageStore(s => s.refresh);
  const starting = usePageStore(s => s.view?.server.state === 'starting');
  useEffect(() => {
    if (!projectId) return;
    void open(projectId);
    return () => close(projectId);
  }, [projectId, open, close]);
  useEffect(() => {
    if (!starting) return;
    const timer = setInterval(() => void refresh(), STARTING_POLL);
    return () => clearInterval(timer);
  }, [starting, refresh]);
}

/** Everything the Page view lays out. */
export function usePageView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  usePageLifecycle(project?.id ?? null);
  const view = usePageStore(s => s.view);
  const error = usePageStore(s => s.error);
  const loading = usePageStore(s => s.loading);
  const tab = usePageStore(s => s.tab);
  const mode = usePageStore(s => s.mode);
  const device = usePageStore(s => s.device);
  const poster = usePageStore(s => s.poster);
  const showStops = usePageStore(s => s.showStops);
  const setTab = usePageStore(s => s.setTab);
  const openInBrowser = usePageStore(s => s.openInBrowser);
  const open = usePageStore(s => s.open);
  const source = frameSource(view, { poster });
  const state: FrameState | null = view ? frameState(view, source.src) : null;
  const running = state?.kind === 'running';
  const concept = view?.settings.source === 'concept';
  return {
    project,
    view,
    error,
    loading,
    retry: () => (project ? void open(project.id) : undefined),
    tab,
    setTab: (id: string) => setTab(id === 'devices' ? 'devices' : 'preview'),
    board: !running ? 'status' : tab === 'devices' ? 'devices' : mode === 'one' ? 'one' : 'compare',
    compareSide: running && tab === 'preview' && mode !== 'one',
    state,
    source,
    device,
    showStops,
    badge: concept ? ('concept' as const) : poster ? ('poster' as const) : ('live' as const),
    site: siteName(view?.siteRepo ?? null),
    command: view?.command?.argv.join(' ') ?? null,
    canOpen: running && !concept,
    concept,
    openInBrowser: () => void openInBrowser(),
    label: concept
      ? 'The page concept'
      : `The ${siteName(view?.siteRepo ?? null)} page from the site dev page`,
  };
}
