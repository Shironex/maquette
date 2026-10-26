import { device as deviceOf } from '@/state/page';
import { DEVICE_BUDGETS, modelChanges, sceneNumbers, verdict } from '@/state/page-numbers';
import { useAppStore } from '@/state/use-app-store';
import { leadReading, usePageStore } from '@/state/use-page-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** This run against what the device may spend, and what changed since the ship. */
export function useDeviceVerdict() {
  const deviceId = usePageStore(s => s.device);
  const models = usePageStore(s => s.inspect?.models);
  const readings = usePageStore(s => s.readings);
  const firstView = usePipelineStore(s => s.view?.firstView.bytes ?? null);
  const setView = useAppStore(s => s.setView);
  const budget = DEVICE_BUDGETS[deviceId];
  const run = models ? sceneNumbers(models, 'shipped') : null;
  const p95 = (readings.ship ?? leadReading(readings)).stats?.p95Ms ?? null;
  const rows = verdict(budget, run, firstView, p95);
  const over = rows.filter(r => r.over);
  const name = deviceOf(deviceId).label;
  const lower = name.toLowerCase();
  const banner =
    rows.length === 0
      ? null
      : over.length
        ? {
            tone: 'warning' as const,
            title: `This run is over the ${lower} budget`,
            detail: over
              .map(r => `${r.label} ${r.value}, the ${lower} allows ${r.budget}.`)
              .join(' '),
          }
        : {
            tone: 'success' as const,
            title: `This run fits the ${lower} budget`,
            detail: `Every number below is inside what a ${budget.label.toLowerCase()} allows.`,
          };
  return {
    name,
    budget,
    banner,
    profile: [
      { label: 'Budget profile', value: budget.label },
      { label: 'Frame time', value: `${budget.frameMs} ms, ${budget.fps} fps` },
      { label: 'First view', value: `${budget.firstViewMb.toFixed(1)} MB` },
      { label: 'GPU memory', value: `${budget.gpuMb} MB` },
      { label: 'Draw calls', value: String(budget.drawCalls) },
    ],
    changes: models ? modelChanges(models) : [],
    openInspector: () => setView('inspector'),
    openPipeline: () => setView('pipeline'),
  };
}
