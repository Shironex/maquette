import {
  Button,
  EmptyState,
  IconButton,
  SegmentedControl,
  cn,
  type SegmentedOption,
} from '@maquette/ui';
import type { InspectView } from '@/ipc/client';
import { num } from '@/state/inspector';
import type { InspectMode } from '@/state/use-inspector-store';
import { SceneBudgetMeter } from '@/features/common/SceneBudgetMeter';
import { CameraStops } from '../CameraStops';
import { CompareMenu } from '../CompareMenu';
import { CompareStage } from '../CompareStage';
import { FrameTime } from '../FrameTime';
import { InspectorShell } from '../InspectorShell';
import { InspectorTabs } from '../InspectorTabs';
import { ModelPanel } from '../ModelPanel';
import { VersionAside, VersionCompare, useCompareWay, useVersionCompare } from '../VersionCompare';
import {
  useCompareSides,
  useCompareStatus,
  useInspectorPage,
  useSelectedModel,
} from './Inspector.hooks';
import { useGameLimits } from './useGameLimits';

const WAYS: SegmentedOption<'before' | 'versions'>[] = [
  { value: 'before', label: 'Before and after' },
  { value: 'versions', label: 'Provider vs provider' },
];

const MODES: SegmentedOption<InspectMode>[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'parts', label: 'Colour by part' },
  { value: 'wireframe', label: 'Wireframe' },
  { value: 'lightmap', label: 'Lightmap' },
];

export interface InspectorProps {
  view: InspectView;
}

/** Two versions side by side in 3D, the camera stops, the frame time and the model's numbers. */
export default function Inspector({ view }: InspectorProps) {
  const { pick, kind, stops, left, right } = useCompareSides(view);
  const status = useCompareStatus(pick);
  const selected = useSelectedModel(view, pick);
  const page = useInspectorPage(view);
  const gameLimits = useGameLimits(selected.model);
  const way = useCompareWay(view, pick, selected.model?.id ?? null);
  const versions = way.value === 'versions';
  const v = useVersionCompare(view, pick, versions, left, right);
  const stage = versions ? v.stage : { kind, stops, left, right };

  const title = pick
    ? versions
      ? pick.comparison.label
      : pick.comparison.kind === 'scene'
        ? 'Scene comparison'
        : `${pick.comparison.label} comparison`
    : 'Inspector';
  const totalMb = num(view.scene.bytes) / 1e6;

  return (
    <InspectorShell
      right={
        <>
          <SceneBudgetMeter value={totalMb} target={num(view.limits.sceneMb)} />
          <Button
            variant="primary"
            icon="play"
            disabled={page.runDisabled}
            disabledReason={page.runReason}
            tooltip="Runs every step, skipping cached ones"
            onClick={page.run}
          >
            Run pipeline
          </Button>
        </>
      }
    >
      <main className="flex min-h-0 flex-1">
        <section className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 py-[18px]">
          <InspectorTabs />
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="max-w-[260px] truncate text-panel-title" title={title}>
              {title}
            </h1>
            {pick ? <CompareMenu view={view} pick={pick} /> : null}
            {way.available ? (
              <SegmentedControl
                label="Compare way"
                options={WAYS}
                value={way.value}
                onChange={way.change}
              />
            ) : null}
            <div className="ml-auto flex items-center gap-2">
              <SegmentedControl
                label="View mode"
                options={MODES}
                value={page.mode}
                onChange={m => {
                  status.clearLightmaps();
                  page.setMode(m);
                }}
              />
              <IconButton
                icon="arrow-clockwise"
                label={page.turntable ? 'Stop rotating' : 'Rotate the view'}
                size="sm"
                variant="secondary"
                aria-pressed={page.turntable}
                onClick={page.toggleTurntable}
                className={cn(page.turntable && 'border-clay-line text-clay-text')}
              />
            </div>
          </div>

          {pick ? (
            <CompareStage
              pick={versions && v.stagePick ? v.stagePick : pick}
              kind={stage.kind}
              left={stage.left}
              right={stage.right}
              turntable={page.turntable}
              stops={stage.stops}
              lightmaps={status.lightmaps}
              ready={status.ready}
              loadError={status.loadError}
              handlers={status.handlers}
              project={view.project}
            />
          ) : (
            <div className="grid min-h-[340px] flex-1 place-items-center rounded-window border border-line">
              <EmptyState
                icon="cube-focus"
                title="Nothing to inspect yet"
                body={page.emptyBody}
                action={
                  <Button
                    icon="flow-arrow"
                    tooltip="Goes to the Pipeline, where the scene is exported"
                    onClick={() => page.setView('pipeline')}
                  >
                    Open the pipeline
                  </Button>
                }
              />
            </div>
          )}

          {versions ? (
            <VersionCompare v={v} />
          ) : (
            <>
              {stops.length > 0 ? (
                <CameraStops view={view} stops={stops} shots={status.shots} />
              ) : null}
              <FrameTime samples={page.samples} />
            </>
          )}
        </section>

        {versions && pick ? (
          <VersionAside v={v} pick={pick} />
        ) : (
          <ModelPanel
            view={view}
            model={selected.model}
            thumb={selected.thumb}
            onPickBudget={selected.onPickBudget}
            onPreset={page.savePreset}
            onExport={selected.onExport}
            exporting={selected.exporting}
            game={gameLimits}
          />
        )}
      </main>
    </InspectorShell>
  );
}
