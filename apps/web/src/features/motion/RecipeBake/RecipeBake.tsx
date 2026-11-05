import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import type { RecipeBakeRecord } from '@/ipc/client';
import {
  STATUS_LOOK,
  addedLine,
  progressOf,
  resultSummary,
  reviewSummary,
} from '@/state/recipe-bake';

/** A running bake: what it is on, how far, and Cancel. */
function Running({
  record,
  assetName,
  busy,
  onCancel,
}: {
  record: RecipeBakeRecord;
  assetName: string;
  busy: boolean;
  onCancel: () => void;
}) {
  const { label, share } = progressOf(record);
  return (
    <div className="flex items-center gap-3">
      <div className="min-w-0 flex-1">
        <div className="mb-1 flex items-center justify-between gap-2 text-meta">
          <span className="truncate">Baking onto {assetName}</span>
          <span className="font-mono text-fg-mute tabular-nums">{label}</span>
        </div>
        <div
          role="progressbar"
          aria-label={`Baking ${record.recipeName}`}
          aria-valuetext={label}
          aria-valuenow={Math.round(share * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
          className="h-1.5 overflow-hidden rounded-full bg-hover"
        >
          <div className="h-full bg-clay" style={{ width: `${Math.round(share * 100)}%` }} />
        </div>
      </div>
      <Button size="sm" icon="stop" loading={busy} onClick={onCancel}>
        Cancel
      </Button>
    </div>
  );
}

/** A finished bake's numbers against their limits, and Review. */
function Done({
  record,
  stale,
  onReview,
}: {
  record: RecipeBakeRecord;
  stale: string | null;
  onReview?: () => void;
}) {
  const result = record.result;
  if (!result) return null;
  const review = record.review ?? null;
  const lines = review?.lines.map(line => {
    const look = STATUS_LOOK[line.status];
    return (
      <li key={line.id} className="flex items-baseline gap-2">
        <Icon
          name={look.icon}
          size={14}
          title={look.words}
          className={cn('flex-none translate-y-0.5', look.tone)}
        />
        <span className="w-24 flex-none text-fg-dim">{line.label}</span>
        <span className="min-w-0 flex-1 truncate">
          <span className="font-mono tabular-nums">{line.value}</span>
          {line.about ? <span className="text-fg-mute"> · {line.about}</span> : null}
        </span>
      </li>
    );
  });
  const warnings = result.warnings.map(w => (
    <p key={w} className="text-meta text-amber">
      {w}
    </p>
  ));
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <b className="font-semibold">Baked</b>
        <span className="truncate font-mono text-[11.5px] text-fg-mute">
          {resultSummary(result)}
        </span>
        {result.mixamo ? (
          <span
            title="A bake of a Mixamo clip stays out of export packs and the clip library."
            className="rounded-pill border border-line px-2 text-[11.5px] leading-5 text-fg-dim"
          >
            Mixamo
          </span>
        ) : null}
        {stale ? (
          <span className="rounded-pill border border-amber px-2 text-[11.5px] leading-5 text-amber">
            Out of date
          </span>
        ) : null}
        <span className="flex-1" />
        {onReview ? (
          <Button
            size="sm"
            variant="primary"
            icon="magnifying-glass"
            tooltip="Plays the bake next to the base clip, with each number against its limit"
            onClick={onReview}
          >
            Review
          </Button>
        ) : null}
      </div>
      {stale ? <p className="text-meta text-amber">{stale}</p> : null}
      {review ? (
        <>
          <p className="text-meta text-fg-mute">{reviewSummary(review)}</p>
          <ul aria-label="Bake result" className="flex list-none flex-col gap-1">
            {lines}
          </ul>
        </>
      ) : null}
      {warnings}
    </div>
  );
}

/** A bake added as a version: which, and what happened to a clip of the same name. */
function Added({ record }: { record: RecipeBakeRecord }) {
  return (
    <div className="flex items-start gap-2">
      <Icon name="check-circle" size={16} className="mt-0.5 flex-none text-sage" />
      <div className="min-w-0 flex-1">
        <p className="font-semibold">{addedLine(record)}</p>
        {record.result?.clipNote ? (
          <p className="text-meta text-fg-mute">{record.result.clipNote}</p>
        ) : null}
      </div>
    </div>
  );
}

/**
 * A recipe's last bake: its progress with Cancel while it runs, its numbers
 * and Review when done, the version it became once added, or why it
 * stopped. Nothing shows before a first bake.
 */
export default function RecipeBake({
  record,
  assetName,
  busy,
  error,
  stale = null,
  onCancel,
  onReview,
}: {
  record: RecipeBakeRecord | null;
  /** The model's name, when the view knows it; the asset id otherwise. */
  assetName?: string | null;
  busy: boolean;
  error: string | null;
  /** Why the bake is out of date: the recipe or the model moved on. */
  stale?: string | null;
  onCancel: () => void;
  /** Opens the finished bake in the review. */
  onReview?: () => void;
}) {
  const refused = error ? (
    <InlineBanner tone="danger" title="The bake did not start" detail={error} />
  ) : null;
  if (!record) return refused;
  const body =
    record.state === 'running' ? (
      <Running
        record={record}
        assetName={assetName ?? record.asset}
        busy={busy}
        onCancel={onCancel}
      />
    ) : record.state === 'done' ? (
      <Done record={record} stale={stale} onReview={onReview} />
    ) : record.state === 'added' ? (
      <Added record={record} />
    ) : record.state === 'cancelled' ? (
      <p className="text-meta text-fg-mute">The last bake was cancelled. Nothing was staged.</p>
    ) : (
      <InlineBanner tone="danger" title="The bake failed" detail={record.error ?? undefined} />
    );
  return (
    <section
      aria-label={`Bake of ${record.recipeName}`}
      className="flex flex-col gap-2 rounded-card border border-line bg-raised px-3 py-2.5"
    >
      {refused}
      {body}
    </section>
  );
}
