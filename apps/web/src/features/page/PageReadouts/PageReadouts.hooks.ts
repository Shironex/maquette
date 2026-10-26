import { device } from '@/state/page';
import { DEVICE_BUDGETS, sceneNumbers } from '@/state/page-numbers';
import { useAppStore } from '@/state/use-app-store';
import { leadReading, usePageStore } from '@/state/use-page-store';

export interface StatCard {
  label: string;
  value: string;
  of: string;
  over: boolean;
}

/** The four numbers of the page as it runs: frame time from the page, the scene from its files. */
export function usePageReadouts() {
  const readings = usePageStore(s => s.readings);
  const models = usePageStore(s => s.inspect?.models);
  const deviceId = usePageStore(s => s.device);
  const source = usePageStore(s => s.view?.settings.source ?? 'site');
  const sceneMb = useAppStore(s => s.workspace?.config.budgets.sceneMb ?? 3);
  const budget = DEVICE_BUDGETS[deviceId];
  const lead = leadReading(readings);
  const stats = lead.stats;
  const site = models ? sceneNumbers(models, 'site') : null;
  const draws = stats?.drawCalls ?? site?.drawCalls ?? null;
  const cards: StatCard[] = [
    {
      label: 'Frame time',
      value: stats ? `${stats.frameMs.toFixed(1)} ms` : 'not sent',
      of: stats ? `p95 ${stats.p95Ms.toFixed(1)}` : '',
      over: Boolean(stats && stats.p95Ms > budget.frameMs),
    },
    {
      label: 'Scene on the page',
      value: site ? `${(site.bytes / 1e6).toFixed(1)} MB` : 'none',
      of: `/ ${sceneMb.toFixed(1)}`,
      over: Boolean(site && site.bytes / 1e6 > sceneMb),
    },
    {
      label: 'GPU memory',
      value: site ? `${Math.round(site.gpuBytes / 1e6)} MB` : 'none',
      of: `/ ${budget.gpuMb}`,
      over: Boolean(site && site.gpuBytes / 1e6 > budget.gpuMb),
    },
    {
      label: stats?.drawCalls != null ? 'Draw calls' : 'Draw calls, est.',
      value: draws === null ? 'none' : String(draws),
      of: `/ ${budget.drawCalls}`,
      over: draws !== null && draws > budget.drawCalls,
    },
  ];
  const at = lead.at === null ? null : Math.round(lead.at) + 1;
  const by = lead.source === 'shim' ? "by Maquette's shim" : "by the site's page bridge";
  const where = `${at === null ? 'In the page' : `Measured ${by} at stop ${at}`} on ${device(deviceId).label}.`;
  const shim =
    lead.source === 'shim'
      ? ' The site has no page bridge, so Maquette measures frame times; draw calls stay estimated.'
      : '';
  const help =
    source === 'concept'
      ? `${where} The scene numbers are the site's models; a page concept shows whatever it holds.`
      : stats
        ? `${where}${shim} The scene is what the site repo holds, the last ship; this run reaches the page when it ships.`
        : `${where} The page sends no frame times yet; the page bridge adds them.`;
  return { cards, help, onSite: Boolean(site) };
}
