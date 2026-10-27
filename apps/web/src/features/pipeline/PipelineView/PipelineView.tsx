import {
  Button,
  Checkbox,
  EmptyState,
  InlineBanner,
  StageStepper,
  Tooltip,
  TopBar,
} from '@maquette/ui';
import { stageViews } from '@/state/pipeline';
import { FirstViewBar } from '@/features/common/FirstViewBar';
import { SceneBudgetMeter } from '@/features/common/SceneBudgetMeter';
import { PipelineGraph } from '../PipelineGraph';
import { BakePanel } from '../BakePanel';
import { GameExportPanel } from '../GameExportPanel';
import { PanelResizer } from '../PanelResizer';
import { PresetsPanel } from '../PresetsPanel';
import { PreviewSiteButton } from '../PreviewSiteButton';
import { RunLog } from '../RunLog';
import { StartersBanner } from '../StartersBanner';
import { ShipView } from '../ShipView';
import { SiteShipView } from '../SiteShipView';
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
    switches: { force, setForce, buildOnly, setBuildOnly },
    textures,
    logStep,
    setLogStep,
    game,
    onDisk,
    saveGame,
    ship,
    panels: { side, fold },
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
  const errorText = errors.map(e => e.message).join(' ');
  const budgetMb = transfer / 1e6;

  return (
    <>
      <TopBar
        lead={<StageStepper stages={stageViews(project.assets ?? [], latest, project.target)} />}
        right={
          <>
            <SceneBudgetMeter value={budgetMb} target={budget} />
            {ship.open ? null : !game ? (
              <>
                <PreviewSiteButton />
                <Button
                  variant="secondary"
                  icon="rocket-launch"
                  tooltip="Sets how Ship writes to the site and shows what it would change"
                  onClick={() => ship.setOpen(true)}
                >
                  Ship settings
                </Button>
              </>
            ) : (
              <Button
                variant={ship.waits ? 'primary' : 'secondary'}
                icon="rocket-launch"
                tooltip={
                  ship.waits
                    ? 'Shows what Ship will write to the game repo, for you to approve'
                    : 'Shows what Ship writes to the game repo'
                }
                onClick={() => ship.setOpen(true)}
              >
                {ship.waits ? 'Review Ship' : 'What Ship changes'}
              </Button>
            )}
            {active ? (
              <Button
                variant="danger"
                icon="x-circle"
                tooltip="Stops the run that is going"
                onClick={() => void cancel()}
              >
                Stop run
              </Button>
            ) : (
              <Button
                variant="primary"
                icon="play"
                disabled={starting || errors.length > 0 || view.def.steps.length === 0}
                disabledReason={
                  errors.length > 0
                    ? 'Fix the pipeline errors first'
                    : view.def.steps.length === 0
                      ? 'The pipeline has no steps'
                      : 'Starting the run'
                }
                tooltip={[
                  force
                    ? 'Runs every step again, ignoring the cache'
                    : 'Runs every step, skipping cached ones',
                  buildOnly ? ', and stops before Ship' : '',
                ].join('')}
                onClick={() => void run({ force, buildOnly })}
              >
                Run pipeline
              </Button>
            )}
          </>
        }
      />
      {game && ship.open ? (
        <ShipView project={project.id} onClose={() => ship.setOpen(false)} />
      ) : ship.open ? (
        <SiteShipView project={project.id} focus={ship.focus} onClose={() => ship.setOpen(false)} />
      ) : (
        <main className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 pt-[18px] pb-[18px]">
          <div className="flex items-start gap-3">
            <div className="min-w-0 flex-1">
              <h1 className="text-screen-title">Pipeline runner</h1>
              <p className="mt-0.5 truncate text-fg-mute">{subtitle}</p>
            </div>
            {game ? null : <FirstViewBar className="mt-0.5 w-56 flex-none" />}
            <Tooltip
              label="Build only"
              detail="Runs every step up to Ship and stops there; nothing is written to the site"
            >
              <span className="mt-2">
                <Checkbox checked={buildOnly} onChange={setBuildOnly} label="Build only" />
              </span>
            </Tooltip>
            <Tooltip
              label="Ignore the cache"
              detail="Runs every step again, even the ones whose inputs did not change"
            >
              <span className="mt-2">
                <Checkbox checked={force} onChange={setForce} label="Ignore the cache" />
              </span>
            </Tooltip>
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
                  <Button
                    size="sm"
                    variant="ghost"
                    tooltip="Opens the log of the step that failed"
                    onClick={() => setLogStep(failedAt ?? 'all')}
                  >
                    View logs
                  </Button>
                  {failedAt ? (
                    <Button
                      size="sm"
                      tooltip="Runs again from the step that failed, keeping the steps before it"
                      onClick={() => void run({ from: failedAt, force, buildOnly })}
                    >
                      Re-run from here
                    </Button>
                  ) : null}
                </>
              }
            />
          ) : null}

          <StartersBanner view={view} busy={active || starting} />

          {/* The steps and the bake card keep their height; the log row takes the rest, then the view scrolls. */}
          <div className="flex-none">
            <PipelineGraph
              view={view}
              busy={active || starting}
              onRunFrom={step => void run({ from: step, force, buildOnly })}
            />
          </div>

          {view.bake ? (
            <BakePanel bake={view.bake} busy={active || starting} onSave={saveBake} fold={fold} />
          ) : null}

          {/* The log takes the height that is left; the presets beside it can be made wider. */}
          <div data-resize-row className="flex min-h-[300px] flex-1">
            <RunLog view={view} step={logStep} onStep={setLogStep} />
            {game ? (
              <div className="ml-3.5 flex">
                <GameExportPanel
                  settings={game}
                  onDisk={onDisk}
                  busy={active || starting}
                  onSave={saveGame}
                />
              </div>
            ) : (
              <>
                <PanelResizer
                  width={side.width}
                  onChange={side.setWidth}
                  onReset={side.reset}
                  label="Compression presets"
                />
                <PresetsPanel
                  view={view}
                  textures={textures}
                  onSave={savePresets}
                  onSizeReport={ship.openReport}
                  style={{ width: side.width }}
                />
              </>
            )}
          </div>
        </main>
      )}
    </>
  );
}
