import { Button, Icon, InlineBanner, cn } from '@maquette/ui';
import type { RecipeBakeRecord } from '@/ipc/client';
import { REVIEW_NEXT, progressOf, resultLines, resultSummary } from '@/state/recipe-bake';

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

/** A finished bake's numbers and what comes next. */
function Done({ record, stale }: { record: RecipeBakeRecord; stale: string | null }) {
  const result = record.result;
  if (!result) return null;
  const lines = resultLines(result).map(line => (
    <li key={line.id} className="flex items-baseline gap-2">
      <Icon
        name={line.bad ? 'warning' : 'check-circle'}
        size={14}
        className={cn('flex-none translate-y-0.5', line.bad ? 'text-amber' : 'text-sage')}
      />
      <span className="w-28 flex-none text-fg-dim">{line.label}</span>
      <span className="min-w-0 flex-1">
        <span className="font-mono tabular-nums">{line.value}</span>
        {line.note ? <span className="block text-meta text-fg-mute">{line.note}</span> : null}
      </span>
    </li>
  ));
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
      </div>
      {stale ? <p className="text-meta text-amber">{stale}</p> : null}
      <ul aria-label="Bake result" className="flex list-none flex-col gap-1">
        {lines}
      </ul>
      {warnings}
      <p className="text-meta text-fg-mute">{REVIEW_NEXT}</p>
    </div>
  );
}

/**
 * A recipe's last bake: its progress with Cancel while it runs, its numbers
 * when done, or why it stopped. Nothing shows before a first bake.
 */
export default function RecipeBake({
  record,
  assetName,
  busy,
  error,
  stale = null,
  onCancel,
}: {
  record: RecipeBakeRecord | null;
  /** The model's name, when the view knows it; the asset id otherwise. */
  assetName?: string | null;
  busy: boolean;
  error: string | null;
  /** Why the bake is out of date: the recipe or the model moved on. */
  stale?: string | null;
  onCancel: () => void;
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
      <Done record={record} stale={stale} />
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
