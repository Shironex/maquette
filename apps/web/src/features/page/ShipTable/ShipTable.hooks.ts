import { compareRows, sceneNumbers } from '@/state/page-numbers';
import { usePageStore } from '@/state/use-page-store';

/** The site's models against this run's, and the way to the compare board. */
export function useShipTable() {
  const models = usePageStore(s => s.inspect?.models);
  const setMode = usePageStore(s => s.setMode);
  const setTab = usePageStore(s => s.setTab);
  const ship = models ? sceneNumbers(models, 'site') : null;
  const run = models ? sceneNumbers(models, 'shipped') : null;
  return {
    rows: ship || run ? compareRows(ship, run) : [],
    loaded: Boolean(models),
    note: !run
      ? 'Run the pipeline to build this run, then compare it with the site.'
      : !ship
        ? 'Nothing is on the site yet: Ship copies this run there.'
        : null,
    compare: () => {
      setTab('preview');
      setMode('ship-run');
    },
  };
}
