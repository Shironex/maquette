import { Suspense } from 'react';
import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import { StageLoader } from '@/features/common/ViewLoader';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import RecipePreview from '@/features/viewer/RecipePreview';
import { fileUrl } from '@/ipc/files';
import type { RecipeReviewLine } from '@/ipc/client';
import { lazyAfterPaint } from '@/lib/after-paint';
import { useKeepClearOfFloating } from '@/lib/useFloatingClearance';
import { STATUS_LOOK, addLabel, resultSummary, reviewSummary } from '@/state/recipe-bake';
import { useRecipeReview, type ReviewShow } from './RecipeReview.hooks';

const MotionStage = lazyAfterPaint(() => import('@/features/viewer/MotionStage'));

const CHIP =
  'pointer-events-none absolute bottom-2 left-2 inline-flex h-6 items-center gap-1.5 rounded-control bg-app/85 px-2 text-[11.5px] leading-none font-medium text-fg';

const SHOWS: { id: ReviewShow; label: string; tooltip: string }[] = [
  { id: 'both', label: 'Side by side', tooltip: 'The bake next to the base clip' },
  { id: 'baked', label: 'Bake', tooltip: 'Only the baked clip on the staged model' },
  { id: 'base', label: 'Base clip', tooltip: 'Only the base clip on the model as it is now' },
];

const NO_LAYERS = [] as const;

/** One number of the review against its limit. */
function Line({ line }: { line: RecipeReviewLine }) {
  const look = STATUS_LOOK[line.status];
  return (
    <li className="flex items-start gap-2 border-t border-line py-2 first:border-t-0">
      <Icon
        name={look.icon}
        size={15}
        title={look.words}
        className={cn('mt-0.5 flex-none', look.tone)}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <b className="font-semibold">{line.label}</b>
          {line.about ? (
            <span className="truncate text-meta text-fg-mute">{line.about}</span>
          ) : null}
        </div>
        <div className="flex items-baseline gap-2 text-meta">
          <span className={cn('font-mono tabular-nums', line.status === 'warn' && 'text-amber')}>
            {line.value}
          </span>
          <span className="text-fg-mute">limit {line.limit}</span>
        </div>
        {line.note ? <p className="text-meta text-fg-mute">{line.note}</p> : null}
      </div>
    </li>
  );
}

/**
 * A finished bake under review: the baked clip on the staged model next to
 * (or instead of) the base clip on the model as it is now, the numbers
 * against their limits, why it is out of date, and Add as vN in a bottom
 * bar the floating cards keep clear of.
 */
export default function RecipeReview() {
  const r = useRecipeReview();
  const bar = useKeepClearOfFloating<HTMLDivElement>();
  const { record, result, review } = r;
  if (!record || !result) {
    return (
      <section aria-label="Recipe review" className="flex flex-1 flex-col gap-3">
        <InlineBanner
          tone="info"
          title="There is no finished bake to review"
          actions={
            <Button size="sm" icon="arrow-left" onClick={r.close}>
              Recipes
            </Button>
          }
        />
      </section>
    );
  }
  const stagedUrl = fileUrl(result.file, String(record.finishedMs ?? ''));
  const baked = r.show !== 'base';
  const base = r.show !== 'baked';
  const label = addLabel(result);
  const blocked = r.stale
    ? 'Bake it again first: this bake is out of date'
    : record.state !== 'done'
      ? 'Only a finished bake can be added'
      : undefined;
  const toggles = SHOWS.map(s => (
    <button
      key={s.id}
      type="button"
      aria-pressed={r.show === s.id}
      title={s.tooltip}
      onClick={() => r.setShow(s.id)}
      className={cn(
        'inline-flex h-7 cursor-pointer items-center rounded-control border border-line px-2.5 text-[12px] leading-none font-medium whitespace-nowrap text-fg-dim hover:bg-hover',
        r.show === s.id && 'border-line-strong bg-hover text-fg'
      )}
    >
      {s.label}
    </button>
  ));
  const lines = review?.lines.map(line => <Line key={line.id} line={line} />);
  return (
    <section aria-label="Recipe review" className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <Button size="sm" icon="arrow-left" tooltip="Back to the recipe list" onClick={r.close}>
          Recipes
        </Button>
        <div className="min-w-0 flex-1">
          <h1 className="text-panel-title truncate">Review {record.recipeName}</h1>
          <p className="truncate font-mono text-[11.5px] text-fg-mute">{resultSummary(result)}</p>
        </div>
        <div role="group" aria-label="Show" className="flex gap-1">
          {toggles}
        </div>
        <Button
          size="sm"
          icon={r.play.playing ? 'pause' : 'play'}
          aria-label={r.play.playing ? 'Pause' : 'Play'}
          tooltip={`Frame ${r.play.frame} of ${r.play.frames}`}
          onClick={r.play.toggle}
        />
      </div>
      <div className="flex min-h-0 flex-1 gap-3">
        <div
          className={cn('grid min-h-[240px] min-w-0 flex-1 gap-3', baked && base && 'grid-cols-2')}
        >
          {baked ? (
            <figure
              aria-label="Baked clip"
              className="relative m-0 overflow-hidden rounded-card border border-line bg-studio"
            >
              {stagedUrl ? (
                <CanvasBoundary resetKey={stagedUrl}>
                  <Suspense fallback={<StageLoader />}>
                    <MotionStage
                      className="absolute! inset-0"
                      label={`${result.clip} baked on ${result.assetName}, staged`}
                      url={stagedUrl}
                      clip={result.clip}
                      time={{ share: r.play.share }}
                      skeleton={false}
                      trail={false}
                      turntable={false}
                      poses={null}
                      outline={[]}
                      onFrame={r.play.tick}
                    />
                  </Suspense>
                </CanvasBoundary>
              ) : (
                <figcaption className="absolute inset-0 grid place-items-center p-3 text-center text-meta text-fg-mute">
                  The staged model cannot be shown here.
                </figcaption>
              )}
              <span className={CHIP}>
                <Icon name="flask" size={13} className="text-fg-dim" />
                Bake: {result.clip} on {result.assetName}, staged
              </span>
            </figure>
          ) : null}
          {base ? (
            <RecipePreview
              project={r.project}
              model={r.model}
              clip={r.clip}
              layers={NO_LAYERS}
              markers={[]}
              share={r.play.share}
              playing={r.show === 'base' && r.play.playing}
              onTick={r.play.tick}
              label={`Base clip on ${r.model?.name ?? result.assetName} as it is now, rough preview`}
              name="Base clip"
            />
          ) : null}
        </div>
        <aside
          aria-label="Review"
          className="flex w-[300px] flex-none flex-col gap-2 overflow-y-auto"
        >
          {r.stale ? (
            <InlineBanner
              tone="warning"
              title="This bake is out of date"
              detail={r.stale}
              actions={
                <Button size="sm" icon="play" loading={r.busy} onClick={() => void r.bakeAgain()}>
                  Bake again
                </Button>
              }
            />
          ) : null}
          {result.clipNote ? (
            <p className="rounded-[7px] border border-line bg-raised px-2.5 py-1.5 text-meta text-fg-dim">
              {result.clipNote}
            </p>
          ) : null}
          {r.approval ? (
            <p className="flex items-start gap-2 rounded-[7px] border border-amber/40 bg-amber-soft px-2.5 py-1.5 text-meta text-fg">
              <Icon name="robot" size={14} className="mt-0.5 flex-none text-amber" />
              An agent asked to add this bake. It waits for your Add.
            </p>
          ) : null}
          {review ? (
            <>
              <p className="text-meta font-medium text-fg-dim">{reviewSummary(review)}</p>
              <ul
                aria-label="Review numbers"
                className="m-0 flex list-none flex-col rounded-card border border-line bg-raised px-3 py-0.5"
              >
                {lines}
              </ul>
              <p className="text-meta text-fg-mute">
                A warn asks for a look; it does not stop the add.
              </p>
            </>
          ) : null}
        </aside>
      </div>
      {r.addError ? (
        <InlineBanner tone="danger" title="The bake was not added" detail={r.addError} />
      ) : null}
      <div
        ref={bar}
        className="flex items-center gap-3 rounded-card border border-line bg-panel px-3 py-2.5"
      >
        <p className="min-w-0 flex-1 text-meta text-fg-dim">
          Adds {result.clip} to {result.assetName} as v{result.fromVersion + 1}
          {result.mixamo ? ', marked as a Mixamo bake' : ''}. The staged model is cleared.
        </p>
        <Button variant="secondary" onClick={r.close}>
          Close
        </Button>
        <Button
          variant="primary"
          icon="check"
          loading={r.busy}
          disabled={Boolean(blocked) || r.busy}
          disabledReason={blocked}
          tooltip="Adds the staged model as the model's next version"
          onClick={r.add}
        >
          {label}
        </Button>
      </div>
    </section>
  );
}
