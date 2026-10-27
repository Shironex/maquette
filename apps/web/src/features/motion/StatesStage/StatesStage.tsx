import { Suspense, lazy } from 'react';
import { Button, EmptyState, Icon, InlineBanner, Select } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { MotionTabs } from '../MotionTabs';
import { PressToggle } from '../PressToggle';
import { StatesStrip } from '../StatesStrip';
import { useStatesStage } from './StatesStage.hooks';

const MotionStage = lazy(() => import('@/features/viewer/MotionStage'));

const CHIP =
  'inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg';

/** The selected model posed in its state, the move into it, and the states strip. */
export default function StatesStage() {
  const stage = useStatesStage();
  const { model, object, state, player } = stage;
  if (!model) return <section className="flex-1" aria-label="States" />;
  const pct = Math.round(player.progress * 100);
  return (
    <section aria-label="States" className="flex min-w-0 flex-1 flex-col gap-3 px-5 py-4">
      <MotionTabs />
      <div className="flex items-center gap-3">
        <h1 className="text-panel-title whitespace-nowrap">{model.name}</h1>
        {object && state ? (
          <Select
            aria-label="State"
            size="sm"
            inline
            options={object.states.map(s => ({ value: s.id, label: s.name }))}
            value={state.id}
            onChange={stage.selectState}
            fieldClassName="w-[150px]"
          />
        ) : null}
        <span className="flex-1" />
        <PressToggle
          icon="selection"
          label="Show parts"
          tooltip="Outlines each separate part of the model"
          pressed={stage.showParts}
          disabled={!object}
          onChange={stage.setShowParts}
        />
      </div>
      <div className="relative min-h-0 flex-1 overflow-hidden rounded-card border border-line bg-studio">
        {stage.url && !stage.failed ? (
          <CanvasBoundary resetKey={stage.url}>
            <Suspense fallback={null}>
              <MotionStage
                className="absolute! inset-0"
                label={stage.label}
                url={stage.url}
                clip={null}
                time={0}
                skeleton={false}
                trail={false}
                turntable={false}
                poses={stage.poses}
                outline={stage.outline}
                onError={stage.setFailed}
              />
            </Suspense>
          </CanvasBoundary>
        ) : null}
        {stage.failed ? (
          <InlineBanner
            tone="danger"
            title="The model could not be shown"
            detail={stage.failed}
            className="absolute inset-x-3 top-3"
          />
        ) : null}
        {stage.badge ? (
          <span className={`${CHIP} pointer-events-none absolute top-3 left-3`}>
            <Icon name="film-strip" size={13} className="text-fg-dim" />
            {stage.badge}
          </span>
        ) : null}
        {stage.transition ? (
          <div className="absolute bottom-3 left-3 flex items-center gap-2.5 rounded-control bg-app/85 px-2 py-1">
            <Button
              size="sm"
              variant="ghost"
              icon="play"
              onClick={player.play}
              tooltip="Plays the model from one state to the next"
              disabled={player.playing}
            >
              Play the move
            </Button>
            <label className="flex items-center gap-2 text-[12px] text-fg-dim">
              Scrub
              <input
                type="range"
                min={0}
                max={100}
                value={pct}
                onChange={e => player.scrub(Number(e.target.value) / 100)}
                className="w-[160px] accent-clay"
              />
            </label>
            <span className="w-9 font-mono text-[11.5px] text-fg">{pct}%</span>
          </div>
        ) : null}
      </div>
      {object ? (
        <StatesStrip />
      ) : (
        <div className="flex flex-none justify-center rounded-card border border-line bg-panel py-6">
          <EmptyState
            icon="selection"
            title={`${model.name} has no states yet`}
            body="A state is a pose of its parts: a door open, a lamp on. The first one is the model as its file has it."
            action={
              <Button
                variant="primary"
                icon="plus"
                tooltip="Adds an at rest state and a moved state to this model"
                onClick={stage.give}
              >
                Give it states
              </Button>
            }
          />
        </div>
      )}
    </section>
  );
}
