import { Suspense, lazy } from 'react';
import { Button, EmptyState, Icon, InlineBanner, SegmentedControl } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { PressToggle } from '@/features/common/PressToggle';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { usePreviewPanel } from './PreviewPanel.hooks';

const VrmStage = lazy(() => import('@/features/viewer/VrmStage'));

const CHIP =
  'inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg';

const POSES = [
  {
    value: 'down',
    label: 'Arms down',
    hint: 'Lowers the arms from the T-pose through the humanoid bones',
  },
  { value: 't-pose', label: 'T-pose', hint: 'The rest pose as written in the file' },
] as const;

/**
 * The written `.vrm` as a VRM app loads it: idling with the arms down, its
 * eyes on the pointer where the file can move them, its spring bones live
 * (Wiggle shakes it so they swing), with what the loader
 * found and the round trip's problems when it failed.
 */
export default function PreviewPanel() {
  const p = usePreviewPanel();
  if (!p.view) return null;
  const help = 'The written .vrm, loaded with three-vrm the way a VRM app loads it.';
  if (!p.url) {
    return (
      <Panel id="avatar-preview" title="Preview" help={help}>
        <EmptyState
          icon="person-arms-spread"
          title={p.written ? 'The written VRM is gone' : 'No VRM written yet'}
          body="Build the avatar on the Export tab, then come back to see it move."
          action={
            <Button icon="arrow-left" tooltip="Opens the Export tab" onClick={p.toExport}>
              Go to Export
            </Button>
          }
        />
      </Panel>
    );
  }
  const chips = p.chips.map(text => (
    <span key={text} className={CHIP}>
      {text}
    </span>
  ));
  return (
    <Panel id="avatar-preview" title="Preview" help={help}>
      {p.roundTripProblems ? (
        <InlineBanner
          tone="danger"
          title="The written file did not read back as a VRM"
          detail={p.roundTripProblems.join(' ')}
        />
      ) : null}
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          label="Pose"
          tone="clay"
          options={POSES}
          value={p.armsDown ? 'down' : 't-pose'}
          onChange={value => p.setArmsDown(value === 'down')}
        />
        <PressToggle
          icon="repeat"
          label="Breathe"
          tooltip="A slow breathing idle in the chest and arms"
          pressed={p.breathe}
          onChange={p.setBreathe}
        />
        <PressToggle
          icon="eye"
          label="Look at pointer"
          tooltip="The eyes follow the pointer over the stage"
          pressed={p.lookAt && !p.stillEyes}
          disabled={Boolean(p.stillEyes)}
          title={p.stillEyes ?? undefined}
          onChange={p.setLookAt}
        />
        <Button
          size="sm"
          variant="secondary"
          icon="arrows-left-right"
          disabled={!p.wiggle.ready}
          tooltip={
            p.wiggle.ready
              ? 'Shakes the hips and the head once, so the springs swing'
              : 'This file has no spring bones to swing'
          }
          onClick={p.wiggle.shake}
        >
          Wiggle
        </Button>
      </div>
      <div className="relative h-[520px] overflow-hidden rounded-card border border-line bg-studio">
        {p.failed ? null : (
          <CanvasBoundary resetKey={p.url}>
            <Suspense fallback={null}>
              <VrmStage
                className="absolute! inset-0"
                label={`${p.name} as a VRM`}
                url={p.url}
                armsDown={p.armsDown}
                breathe={p.breathe}
                lookAt={p.lookAt}
                wiggle={p.wiggle.count}
                onLoaded={p.onLoaded}
                onError={p.onError}
              />
            </Suspense>
          </CanvasBoundary>
        )}
        {p.failed ? (
          <InlineBanner
            tone="danger"
            title="The VRM could not be loaded"
            detail={p.failed}
            className="absolute inset-x-3 top-3"
          />
        ) : null}
        <div className="pointer-events-none absolute top-3 left-3 flex flex-wrap gap-1.5">
          {chips}
        </div>
        <span className={`${CHIP} pointer-events-none absolute bottom-3 left-3 text-fg-dim`}>
          <Icon name="hand-pointing" size={13} />
          Drag to orbit · scroll to zoom
        </span>
      </div>
      {p.missing.length > 0 ? (
        <InlineBanner
          tone="danger"
          title="The humanoid lacks required bones"
          detail={`three-vrm found no ${p.missing.join(', ')}.`}
        />
      ) : null}
      {p.stillEyes ? (
        <InlineBanner tone="info" title="The eyes stay still" detail={p.stillEyes} />
      ) : null}
    </Panel>
  );
}
