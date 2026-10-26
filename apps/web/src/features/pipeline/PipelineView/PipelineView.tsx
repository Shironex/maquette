import { Button, Checkbox, EmptyState, InlineBanner, StageStepper, TopBar } from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { stageViews } from '@/state/pipeline';
import { FirstViewBar } from '@/features/common/FirstViewBar';
import { SceneBudgetMeter } from '@/features/common/SceneBudgetMeter';
import { PipelineGraph } from '../PipelineGraph';
import { BakePanel } from '../BakePanel';
import { GameExportPanel } from '../GameExportPanel';
import { PresetsPanel } from '../PresetsPanel';
import { PreviewSiteButton } from '../PreviewSiteButton';
import { RunLog } from '../RunLog';
import { StartersBanner } from '../StartersBanner';
import { ShipView } from '../ShipView';
import { runOverview } from './runOverview';
import { usePipelineView } from './PipelineView.hooks';

export default function PipelineView() {
  const {
    project,
    budget,
    view,
    loading,
    error,
    starting,
    run,
    cancel,
    savePresets,
    saveBake,
    load,
    force,
    setForce,
    logStep,
    setLogStep,
    game,
    onDisk,
    saveGame,
    ship,
  } = usePipelineView();

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

  const { latest, active, transfer, failedAt, failedName, errors, subtitle } = runOverview(view);
  const artifactCards = view.models.slice(0, 8).map(m => (
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
  ));
  const errorText = errors.map(e => e.message).join(' ');
  const budgetMb = transfer / 1e6;

  return (
    <>
      <TopBar
        lead={<StageStepper stages={stageViews(project.assets ?? [], latest, project.target)} />}
        right={
          <>
            <SceneBudgetMeter value={budgetMb} target={budget} />
            {!game ? (
              <PreviewSiteButton />
            ) : ship.open ? null : (
              <Button
                variant={ship.waits ? 'primary' : 'secondary'}
                icon="rocket-launch"
                onClick={() => ship.setOpen(true)}
              >
                {ship.waits ? 'Review Ship' : 'What Ship changes'}
              </Button>
            )}
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
      {game && ship.open ? (
        <ShipView project={project.id} onClose={() => ship.setOpen(false)} />
      ) : (
        <main className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 pt-[18px] pb-[18px]">
          <div className="flex items-start gap-3">
            <div className="min-w-0 flex-1">
              <h1 className="text-screen-title">Pipeline runner</h1>
              <p className="mt-0.5 truncate text-fg-mute">{subtitle}</p>
            </div>
            {game ? null : <FirstViewBar className="mt-0.5 w-56 flex-none" />}
            <Checkbox
              checked={force}
              onChange={setForce}
              label="Ignore the cache"
              className="mt-2"
            />
          </div>

          {errors.length > 0 ? (
            <InlineBanner
              tone="warning"
              title="pipeline.json needs a fix before it can run"
              detail={errorText}
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

          {/* The steps and the bake settings keep their height; the log row gives way, then the view scrolls. */}
          <div className="flex-none">
            <PipelineGraph
              view={view}
              busy={active || starting}
              onRunFrom={step => void run({ from: step, force })}
            />
          </div>

          {view.bake ? (
            <BakePanel bake={view.bake} busy={active || starting} onSave={saveBake} />
          ) : null}

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
              <div className="grid grid-cols-4 gap-2.5">{artifactCards}</div>
            </section>
          ) : null}

          <div className="flex min-h-[220px] flex-1 gap-3.5">
            <RunLog view={view} step={logStep} onStep={setLogStep} />
            {game ? (
              <GameExportPanel
                settings={game}
                onDisk={onDisk}
                busy={active || starting}
                onSave={saveGame}
              />
            ) : (
              <PresetsPanel view={view} onSave={savePresets} />
            )}
          </div>
        </main>
      )}
    </>
  );
}
