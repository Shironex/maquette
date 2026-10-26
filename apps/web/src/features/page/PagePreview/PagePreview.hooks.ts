import { frameSource, frameState, siteName } from '@/state/page';
import { useAppStore } from '@/state/use-app-store';
import { leadReading, usePageStore } from '@/state/use-page-store';
import { usePageLifecycle } from '../PageView/PageView.hooks';
import { usePlayback } from '../StopScrubber/StopScrubber.hooks';

/**
 * The project's page for another view to scrub: it starts the dev server
 * as the Page view does and lets go of it when the view closes, and moves
 * the page on at reading speed while Play is on.
 */
export function usePagePreview() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  usePageLifecycle(project?.id ?? null);
  const view = usePageStore(s => s.view);
  const device = usePageStore(s => s.device);
  const poster = usePageStore(s => s.poster);
  const lead = leadReading(usePageStore(s => s.readings));
  const cameraStops = usePageStore(s => s.inspect?.stops.stops.length ?? 0);
  usePlayback(lead.stops?.length || cameraStops);
  const source = frameSource(view, { poster });
  const state = view ? frameState(view, source.src) : null;
  const site = siteName(view?.siteRepo ?? null);
  const concept = view?.settings.source === 'concept';
  return {
    loading: !view,
    source,
    device,
    status: state && state.kind !== 'running' ? state : null,
    site,
    command: view?.command?.argv.join(' ') ?? null,
    badge: concept ? ('concept' as const) : poster ? ('poster' as const) : ('live' as const),
    label: concept ? 'The page concept' : `The ${site} page from the site dev page`,
  };
}
