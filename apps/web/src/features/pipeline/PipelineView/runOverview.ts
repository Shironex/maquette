import type { PipelineView } from '@/ipc/client';
import { estimatedTransfer, failedStep, runSummary } from '@/state/pipeline';

function clock(ms: number | null | undefined): string {
  if (!ms) return '';
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** What the runner header and banners say about the latest run. */
export function runOverview(view: PipelineView) {
  const latest = view.runs[0];
  const active = view.active !== null || view.queued.length > 0;
  const transfer = estimatedTransfer(view);
  const failedAt = latest?.state === 'failed' ? failedStep(latest) : null;
  const failedName = latest?.steps.find(s => s.id === failedAt)?.name;
  const errors = view.issues.filter(i => i.level === 'error');
  const subtitle = latest
    ? [
        `Run ${latest.number}`,
        latest.force ? 'forced' : null,
        latest.from
          ? `from ${latest.steps.find(s => s.id === latest.from)?.name ?? latest.from}`
          : null,
        latest.startedAtMs ? `started ${clock(latest.startedAtMs)}` : null,
        `${view.def.steps.length} steps`,
        runSummary(latest),
      ]
        .filter(Boolean)
        .join(' · ')
    : `No runs yet · ${view.def.steps.length} steps · runs in ${view.root}`;
  return { latest, active, transfer, failedAt, failedName, errors, subtitle };
}
