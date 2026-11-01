import { Suspense } from 'react';
import { StageLoader, useStageReady } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
import { Button, Icon, InlineBanner, Select } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { ClipTimeline } from '../ClipTimeline';
import { MotionTabs } from '../MotionTabs';
import { NoClipsOptions } from '../NoClipsOptions';
import { PressToggle } from '@/features/common/PressToggle';
import { useClipStage } from './ClipStage.hooks';

const MotionStage = lazyAfterPaint(() => import('@/features/viewer/MotionStage'));

const CHIP =
  'inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg';

/** The selected model playing its clip, with the overlays and the timeline under it. */
export default function ClipStage() {
  const stage = useClipStage();
  const shown = useStageReady(stage.url);
  const waiting = stage.url && !stage.failed && !shown.ready;
  const { model, clip, overlays } = stage;
  if (!model) return <section className="flex-1" aria-label="Preview" />;
  const clipOptions = model.clips.map(c => ({ value: c.name, label: c.name }));
  const badges = stage.badges.map(b => (
    <span key={b.text} className={CHIP}>
      <Icon name={b.icon} size={13} className="text-fg-dim" />
      {b.text}
    </span>
  ));
  return (
    <section aria-label="Preview" className="flex min-w-0 flex-1 flex-col gap-3 px-5 py-4">
      <MotionTabs />
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h1 className="text-panel-title whitespace-nowrap">{model.name}</h1>
        {model.clips.length > 0 ? (
          <Select
            aria-label="Clip"
            size="sm"
            inline
            options={clipOptions}
            value={clip?.name ?? ''}
            onChange={stage.selectClip}
            title={clip?.name}
            fieldClassName="w-[150px]"
          />
        ) : (
          <span className="inline-flex h-[26px] items-center rounded-control border border-line px-2.5 text-[12px] text-fg-mute">
            No clips
          </span>
        )}
        <span className="flex-1" />
        <PressToggle
          icon="bone"
          label="Skeleton"
          tooltip="Shows the bones over the model"
          pressed={overlays.skeleton && Boolean(model.rig)}
          disabled={!model.rig}
          title={model.rig ? undefined : 'This model has no rig'}
          onChange={overlays.setSkeleton}
        />
        <PressToggle
          icon="path"
          label="Root trail"
          tooltip="Draws the line the model's root travels along"
          pressed={overlays.trail}
          onChange={overlays.setTrail}
        />
        <PressToggle
          icon="arrows-clockwise"
          label="Turntable"
          tooltip="Turns the model slowly so you can see all sides"
          pressed={overlays.turntable}
          onChange={overlays.setTurntable}
        />
      </div>
      {stage.bake ? (
        <div className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2">
          <Icon name="books" size={15} className="flex-none text-fg-dim" />
          <div className="min-w-0 flex-1">
            <p className="text-[13px] leading-[18px] font-semibold">{stage.bake.title}</p>
            <p className="text-meta text-fg-mute">{stage.bake.detail}</p>
          </div>
          <Button
            size="sm"
            icon="arrows-left-right"
            tooltip="Opens both versions side by side in the Inspector"
            onClick={stage.bake.compare}
          >
            {stage.bake.compareLabel}
          </Button>
        </div>
      ) : null}
      <div className="relative min-h-0 flex-1 overflow-hidden rounded-card border border-line bg-studio">
        {stage.url && !stage.failed ? (
          <CanvasBoundary resetKey={stage.url}>
            <Suspense fallback={<StageLoader />}>
              <MotionStage
                className="absolute! inset-0"
                label={stage.label}
                url={stage.url}
                clip={clip?.index ?? null}
                time={stage.time}
                skeleton={overlays.skeleton}
                trail={overlays.trail}
                turntable={overlays.turntable}
                poses={null}
                outline={[]}
                onFrame={stage.tick}
                report={{ onError: stage.setFailed, onLoaded: shown.onLoaded }}
              />
            </Suspense>
          </CanvasBoundary>
        ) : null}
        {waiting ? <StageLoader /> : null}
        {stage.failed ? (
          <InlineBanner
            tone="danger"
            title="The model could not be shown"
            detail={stage.failed}
            className="absolute inset-x-3 top-3"
          />
        ) : null}
        <div className="pointer-events-none absolute top-3 left-3 flex gap-1.5">{badges}</div>
        <span className={`${CHIP} pointer-events-none absolute bottom-3 left-3 text-fg-dim`}>
          <Icon name="hand-pointing" size={13} />
          Drag to orbit · scroll to zoom
        </span>
      </div>
      {clip ? <ClipTimeline /> : <NoClipsOptions />}
    </section>
  );
}
