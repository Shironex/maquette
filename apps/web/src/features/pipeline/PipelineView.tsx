import { useState } from 'react';
import {
  BudgetMeter,
  Button,
  Checkbox,
  EmptyState,
  InlineBanner,
  StageStepper,
  TopBar,
} from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { useAppStore } from '@/state/app-store';
import { usePipelineStore } from '@/state/pipeline-store';
import { estimatedTransfer, failedStep, runSummary, stageViews } from '@/state/pipeline';
import { PipelineGraph } from './PipelineGraph';
import { PresetsPanel } from './PresetsPanel';
import { RunLog } from './RunLog';
import { StartersBanner } from './StartersBanner';

function clock(ms: number | null | undefined): string {
  if (!ms) return '';
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

export function PipelineView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const budget = useAppStore(s => s.workspace?.config.budgets.sceneMb ?? 3);
  const { view, loading, error, starting, run, cancel, savePresets, load } = usePipelineStore();
  const [force, setForce] = useState(false);
  const [logStep, setLogStep] = useState('all');

  if (!project) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Pipeline</h1>} />
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="flow-arrow"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </>
    );
  }
  if (!view) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Pipeline</h1>} />
        <main className="grid flex-1 place-items-center" aria-busy={loading}>
          {error ? (
            <EmptyState
              icon="warning-circle"
              title="The pipeline could not be read"
              body={error}
              action={
                <Button icon="arrow-clockwise" onClick={() => void load(project.id)}>
                  Try again
                </Button>
              }
            />
          ) : null}
        </main>
      </>
    );
  }

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

  return (
    <>
      <TopBar
        lead={<StageStepper stages={stageViews(project.assets ?? [], latest)} />}
        right={
          <>
            <BudgetMeter label="Scene budget" value={transfer / 1e6} target={budget} unit="MB" />
            {active ? (
              <Button variant="danger" icon="x-circle" onClick={() => void cancel()}>
                Stop run
              </Button>
            ) : (
              <Button
                variant="primary"
                icon="play"
                disabled={starting || errors.length > 0 || view.def.steps.length === 0}
                onClick={() => void run({ force })}
              >
                Run pipeline
              </Button>
            )}
          </>
        }
      />
      <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 pt-[18px] pb-[18px]">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <h1 className="text-screen-title">Pipeline runner</h1>
            <p className="mt-0.5 truncate text-fg-mute">{subtitle}</p>
          </div>
          <Checkbox checked={force} onChange={setForce} label="Ignore the cache" className="mt-2" />
        </div>

        {errors.length > 0 ? (
          <InlineBanner
            tone="warning"
            title="pipeline.json needs a fix before it can run"
            detail={errors.map(e => e.message).join(' ')}
          />
        ) : null}
        {latest?.state === 'failed' && !active ? (
          <InlineBanner
            tone="danger"
            title={`Run ${latest.number} failed${failedName ? ` at ${failedName}` : ''}`}
            detail={latest.message ?? undefined}
            actions={
              <>
                <Button size="sm" variant="ghost" onClick={() => setLogStep(failedAt ?? 'all')}>
                  View logs
                </Button>
                {failedAt ? (
                  <Button size="sm" onClick={() => void run({ from: failedAt, force })}>
                    Re-run from here
                  </Button>
                ) : null}
              </>
            }
          />
        ) : null}

        <StartersBanner view={view} busy={active || starting} />

        <PipelineGraph
          view={view}
          busy={active || starting}
          onRunFrom={step => void run({ from: step, force })}
        />

        {view.artifacts.length > 0 ? (
          <section aria-labelledby="artifacts-title" className="flex flex-col gap-2">
            <div className="flex items-baseline justify-between">
              <h2
                id="artifacts-title"
                className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
              >
                Generated artifacts
              </h2>
              <span className="text-meta text-fg-mute">raw export to compressed</span>
            </div>
            <div className="grid grid-cols-4 gap-2.5">
              {view.models.slice(0, 8).map(m => (
                <div
                  key={m.model}
                  className="flex min-w-0 flex-col gap-0.5 rounded-card border border-line bg-raised px-3 py-2"
                >
                  <span className="truncate font-mono text-[12px]">{m.model}.glb</span>
                  <span className="font-mono text-[11.5px] text-fg-mute">
                    {m.rawBytes ? formatBytes(m.rawBytes) : 'not exported'}
                    {m.shippedBytes ? ` → ${formatBytes(m.shippedBytes)}` : ''}
                  </span>
                </div>
              ))}
            </div>
          </section>
        ) : null}

        <div className="flex min-h-[220px] flex-1 gap-3.5">
          <RunLog view={view} step={logStep} onStep={setLogStep} />
          <PresetsPanel view={view} onSave={savePresets} />
        </div>
      </main>
    </>
  );
}
