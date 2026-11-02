import { Button, cn, InlineBanner } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { countsLine, fieldsLine } from '@/state/plan-camera';
import { useApplyReview } from './ApplyReview.hooks';

const SIGN = { add: '+', remove: '-', same: ' ' } as const;

/**
 * What Apply writes to `layout.json`, shown before it does: the order
 * after Apply with every stop that moves, comes, goes or changes, and the
 * stops of `layout.json` line by line (a moved stop leaves its old place
 * and comes in at its new one). Apply refuses when `layout.json` or the
 * plan changed since this was shown, and says to review again.
 */
export default function ApplyReview() {
  const v = useApplyReview();
  if (!v.review) return null;
  const { diff } = v.review;
  const order = diff.stops.map(s => (
    <tr key={s.id} className={cn(s.removed && 'text-danger')}>
      <td className="py-0.5 pr-2 text-right font-mono text-fg-mute">{s.to ?? ''}</td>
      <td className="py-0.5 pr-2 text-fg">{s.name}</td>
      <td className="py-0.5 text-fg-dim">
        {s.added
          ? 'new stop'
          : s.removed
            ? 'removed'
            : s.moved
              ? `moves from ${s.from}`
              : s.from !== s.to
                ? `now ${s.to}, was ${s.from}`
                : 'same'}
        {fieldsLine(s.fields, true)}
      </td>
    </tr>
  ));
  const lines = diff.lines.map((line, i) => (
    <div key={i} className={cn('plan-diff-line', `is-${line.kind}`)}>
      <span aria-hidden="true">{SIGN[line.kind]}</span>
      <span className="sr-only">
        {line.kind === 'same' ? '' : `${line.kind === 'add' ? 'Added' : 'Removed'}: `}
      </span>
      <code className="min-w-0 flex-1 break-all">{line.text}</code>
      {line.note ? <span className="plan-diff-note">{line.note}</span> : null}
    </div>
  ));
  return (
    <Sheet
      id="apply-review"
      icon="upload-simple"
      title="Apply to layout.json"
      meta={countsLine(diff)}
      width={600}
      onClose={v.close}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        {v.error ? (
          <InlineBanner
            tone="danger"
            title="Nothing was applied"
            detail={v.error}
            actions={
              <Button size="sm" onClick={v.again}>
                Review again
              </Button>
            }
          />
        ) : null}
        {!v.error && v.problems.length ? (
          <InlineBanner
            tone="warning"
            title="This can not be applied yet"
            detail={v.problems.join(' ')}
          />
        ) : null}
        <section aria-label="Order this Apply writes">
          <h3 className="pb-1.5 text-[13px] font-semibold text-fg">Order after Apply</h3>
          <table className="w-full text-[12.5px]">
            <tbody>{order}</tbody>
          </table>
        </section>
        <section aria-label="Changes to layout.json">
          <h3 className="pb-1.5 text-[13px] font-semibold text-fg">Changes to layout.json</h3>
          <div className="plan-diff">{lines}</div>
          {v.review.dropped ? (
            <p className="pt-1.5 text-[12px] text-amber">
              {v.review.dropped} trigger{v.review.dropped === 1 ? '' : 's'}, timeline or path key
              {v.review.dropped === 1 ? '' : 's'} wait for a removed stop and go with it.
            </p>
          ) : null}
        </section>
        <p className="text-[12px] text-fg-mute">
          Apply changes only the stops, and the triggers, timeline keys and path keys that wait for
          a stop it removes. The old file is kept in plans/history, and Undo on the Apply line in
          Changes puts it back while nothing changed it since.
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
        <Button onClick={v.close}>Cancel</Button>
        <Button
          variant="primary"
          loading={v.applying}
          disabled={v.off !== null}
          disabledReason={v.off ?? undefined}
          onClick={v.apply}
        >
          Apply
        </Button>
      </div>
    </Sheet>
  );
}
