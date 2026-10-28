import { Suspense, lazy } from 'react';
import { Icon, KeyValueList, cn, type IconName } from '@maquette/ui';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { ReviewStatus } from '@/ipc/client';
import { useReviewStep } from './ReviewStep.hooks';

const MotionStage = lazy(() => import('@/features/viewer/MotionStage'));

const STATUS: Record<ReviewStatus, { label: string; icon: IconName; className: string }> = {
  good: { label: 'good', icon: 'check-circle', className: 'text-sage' },
  look: { label: 'to look at', icon: 'warning', className: 'text-amber' },
  failed: { label: 'failed', icon: 'x-circle', className: 'text-danger' },
};

function Stage({
  url,
  clip,
  time,
  label,
  onFrame,
}: {
  url: string | null;
  clip: number;
  time: number;
  label: string;
  onFrame?: (delta: number) => void;
}) {
  return (
    <figure className="flex min-w-0 flex-1 flex-col gap-1.5">
      <div className="relative min-h-[260px] flex-1 overflow-hidden rounded-card border border-line bg-studio">
        {url ? (
          <CanvasBoundary resetKey={`${url}#${clip}`}>
            <Suspense fallback={null}>
              <MotionStage
                className="absolute! inset-0"
                label={label}
                url={url}
                clip={clip}
                time={time}
                skeleton
                trail
                turntable={false}
                poses={null}
                outline={[]}
                onFrame={onFrame}
              />
            </Suspense>
          </CanvasBoundary>
        ) : null}
      </div>
      <figcaption className="text-meta text-fg-dim">{label}</figcaption>
    </figure>
  );
}

/** Step four: the bake graded clip by clip, failed clips left out of the version. */
export default function ReviewStep() {
  const step = useReviewStep();
  const { draft, selected, counts } = step;
  if (!draft || !counts) return null;
  const list = step.clips.map(c => {
    const s = STATUS[c.status];
    const on = selected?.clip === c.clip;
    return (
      <li key={c.clip}>
        <button
          type="button"
          aria-pressed={on}
          onClick={() => step.select(c.clip)}
          className={cn(
            'flex w-full cursor-pointer items-start gap-2 rounded-control px-2.5 py-2 text-left hover:bg-hover',
            on && 'bg-clay-soft'
          )}
        >
          <Icon name={s.icon} size={15} className={cn('mt-0.5 flex-none', s.className)} />
          <span className="min-w-0">
            <span className="block font-mono text-[12.5px]">{c.name}</span>
            <span className="block text-meta text-fg-dim">{c.note}</span>
          </span>
          <span className="sr-only">, {s.label}</span>
        </button>
      </li>
    );
  });
  const failed = draft.review.clips.filter(c => c.status === 'failed').map(c => c.name);
  const facts = [
    { label: 'Bones driven', value: `${draft.review.driven}` },
    {
      label: 'Limbs turned',
      value: `${Math.round(draft.review.alignedDeg ?? 0)}° to the clip's rest pose`,
    },
    { label: 'Hips moves', value: `x ${(draft.review.hipsScale ?? 1).toFixed(2)} by height` },
    {
      label: 'At rest',
      value: draft.review.atRest.length ? draft.review.atRest.join(', ') : 'none',
    },
  ];
  return (
    <>
      <aside
        aria-label="Baked clips"
        className="flex w-[330px] flex-none flex-col gap-2 overflow-y-auto border-r border-line p-3"
      >
        <p className="px-1 font-mono text-[11.5px] text-fg-mute">
          {draft.review.clips.length} clips · ran {Math.round(draft.review.seconds ?? 0)} s ·{' '}
          {counts.good} good · {counts.look} to look at · {counts.failed} failed
        </p>
        <ul className="flex flex-col gap-0.5">{list}</ul>
      </aside>
      <section aria-label="Side by side" className="flex min-w-0 flex-1 flex-col gap-3 p-4">
        {selected ? (
          <>
            <p className="text-[13px]">
              <span className="font-mono">{selected.name}</span>
              <span className="text-fg-dim">
                {' '}
                from {step.source?.clip.name ?? selected.clip} · {selected.frames} f · frame{' '}
                {step.frame}
              </span>
            </p>
            <div className="flex min-h-0 flex-1 gap-3">
              <Stage
                url={step.sourceUrl}
                clip={0}
                time={step.time}
                label={`Source, ${step.source?.clip.rig.kind ?? 'clip'} skeleton`}
                onFrame={step.tick}
              />
              <Stage
                url={step.draftUrl}
                clip={step.draftClip}
                time={step.time}
                label={`On ${draft.name}, v${draft.fromVersion + 1} draft`}
              />
            </div>
            <p className="text-meta text-fg-dim">{selected.note}.</p>
          </>
        ) : null}
        <KeyValueList items={facts} />
        {failed.length ? (
          <p className="text-meta text-fg-mute">
            {failed.join(', ')} {failed.length === 1 ? 'is' : 'are'} left out until fixed in the
            library.
          </p>
        ) : null}
      </section>
    </>
  );
}
