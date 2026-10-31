import { fileUrl } from '@/ipc/files';
import { useNow } from '@/lib/useNow';
import { formatAgo, formatBytes } from '@/lib/format';
import { frameSource, nearestStop, siteName, type DeviceId } from '@/state/page';
import { compareRows, sceneNumbers } from '@/state/page-numbers';
import { leadReading, usePageStore, type Reading } from '@/state/use-page-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import type { CompareColumnView } from './CompareBoard.types';

const frameRow = (reading: Reading | undefined) => ({
  label: 'Frame time, p95',
  value: reading?.stats ? `${reading.stats.p95Ms.toFixed(1)} ms` : 'not sent',
});

/** The columns of the compare board: the site now, this run, and the page without WebGL. */
export function useCompareBoard(): { columns: CompareColumnView[]; device: DeviceId } {
  const view = usePageStore(s => s.view);
  const models = usePageStore(s => s.inspect?.models);
  const cameraStops = usePageStore(s => s.inspect?.stops.stops);
  const mode = usePageStore(s => s.mode);
  const device = usePageStore(s => s.device);
  const readings = usePageStore(s => s.readings);
  const target = usePageStore(s => s.target);
  const latest = usePipelineStore(s => s.view?.runs[0]);
  const firstView = usePipelineStore(s => s.view?.firstView.bytes ?? null);
  const now = useNow(30_000);
  const ship = models ? sceneNumbers(models, 'site') : null;
  const run = models ? sceneNumbers(models, 'shipped') : null;
  const rows = compareRows(ship, run);
  const [size, tris, gpu, draws] = rows;
  const shipped = (models ?? [])
    .filter(m => m.source === 'site' && m.modifiedAt)
    .map(m => Date.parse(m.modifiedAt ?? ''))
    .sort((a, b) => b - a)[0];
  const live = frameSource(view);
  const poster = frameSource(view, { poster: true });
  const lead = leadReading(readings);
  const stopIndex = nearestStop(lead.at ?? target, cameraStops?.length ?? 0);
  const still = fileUrl(cameraStops?.[stopIndex]?.poster ?? null);
  const site = siteName(view?.siteRepo ?? null);

  const columns: CompareColumnView[] = [
    {
      id: 'ship',
      title: 'Current ship',
      sub: shipped
        ? `${site} · shipped ${formatAgo(shipped, now)}`
        : `${site} · nothing shipped yet`,
      src: live.src,
      image: null,
      address: live.address,
      badge: 'live',
      empty: live.src ? null : 'The dev server is not running.',
      rows: [
        { label: 'Scene size', value: size?.ship ?? 'none' },
        { label: 'Triangles', value: tris?.ship ?? 'none' },
        frameRow(readings.ship),
        { label: 'GPU memory', value: gpu?.ship ?? 'none' },
        {
          label: readings.ship?.stats?.drawCalls != null ? 'Draw calls' : 'Draw calls, est.',
          value: String(readings.ship?.stats?.drawCalls ?? draws?.ship ?? 'none'),
        },
      ],
    },
    {
      id: 'run',
      title: 'This run',
      sub: latest
        ? `Run ${latest.number} · built ${formatAgo(latest.finishedAtMs ?? latest.queuedAtMs ?? now, now)}`
        : 'No run yet',
      next: true,
      src: null,
      image: still,
      address: `poster, stop ${stopIndex + 1}`,
      badge: 'run',
      empty: still ? null : 'No poster for this stop yet: the Posters step renders one per stop.',
      rows: [
        ...rows.map(r => ({
          label: r.label,
          value: r.run,
          change: r.better === null ? undefined : r.change,
          better: r.better,
        })),
        // A still has no frame time: the page measures it once this run ships.
        { label: 'Frame time, p95', value: 'after it ships' },
      ],
    },
  ];
  if (mode === 'poster') {
    columns.push({
      id: 'poster',
      title: 'Poster fallback',
      sub: 'The page without WebGL',
      src: poster.src,
      image: null,
      address: poster.address,
      badge: 'poster',
      empty: poster.src ? null : 'The dev server is not running.',
      rows: [
        { label: 'Shows when', value: 'no WebGL' },
        { label: 'Stop', value: cameraStops?.[stopIndex]?.name ?? 'none' },
        {
          label: 'Swaps to 3D',
          value: firstView === null ? 'with the page' : `at ${formatBytes(firstView)}`,
        },
        { label: '3D download', value: 'none' },
        readings.poster?.stats
          ? frameRow(readings.poster)
          : { label: 'Frame time, p95', value: 'no 3D' },
        { label: 'Draw calls', value: String(readings.poster?.stats?.drawCalls ?? 0) },
      ],
    });
  }
  return { columns, device };
}
