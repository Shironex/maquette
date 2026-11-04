import { Button, cn } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { countsLine } from '@/state/plan-camera';
import { useCameraCompare } from './CameraCompare.hooks';

const SIGN = { add: '+', remove: '-', same: ' ' } as const;

/**
 * Both sides after `layout.json` changed under the camera plan: what
 * changed there since the plan was based on it, and what a rebase takes in,
 * keeps from the plan and could not settle. Nothing is written until
 * Rebase or Start over.
 */
export default function CameraCompare() {
  const v = useCameraCompare();
  if (!v.report) return null;
  const { report } = v;
  const list = (title: string, items: string[], tone?: string) => {
    if (!items.length) return null;
    const rows = items.map(item => <li key={item}>{item}</li>);
    return (
      <section aria-label={title}>
        <h3 className="pb-1 text-[13px] font-semibold text-fg">{title}</h3>
        <ul className={cn('flex list-disc flex-col gap-0.5 pl-5 text-[12.5px]', tone)}>{rows}</ul>
      </section>
    );
  };
  const lines = (v.outside?.lines ?? []).map((line, i) => (
    <div key={i} className={cn('plan-diff-line', `is-${line.kind}`)}>
      <span aria-hidden="true">{SIGN[line.kind]}</span>
      <code className="min-w-0 flex-1 break-all">{line.text}</code>
      {line.note ? <span className="plan-diff-note">{line.note}</span> : null}
    </div>
  ));
  const none = !report.taken.length && !report.added.length && !report.removed.length;
  return (
    <Sheet
      id="camera-compare"
      icon="git-branch"
      title="layout.json changed"
      meta={v.outside ? countsLine(v.outside) : undefined}
      width={560}
      onClose={v.close}
      footer={
        <div className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button onClick={v.close}>Cancel</Button>
          {v.error ? (
            <Button variant="primary" icon="arrow-clockwise" onClick={v.reviewAgain}>
              Review again
            </Button>
          ) : (
            <>
              <Button variant="danger" disabled={v.busy} onClick={v.startOver}>
                Start over
              </Button>
              <Button variant="primary" loading={v.busy} onClick={v.rebase}>
                Rebase
              </Button>
            </>
          )}
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        <section aria-label="What changed in layout.json">
          <h3 className="pb-1.5 text-[13px] font-semibold text-fg">What changed in layout.json</h3>
          <div className="plan-diff">{lines}</div>
        </section>
        {list('A rebase takes in', report.taken)}
        {list('Stops it adds to the plan', report.added)}
        {list('Stops it takes out of the plan', report.removed)}
        {list('Kept from the plan, changed on both sides', report.clashes, 'text-amber')}
        {none && !report.clashes.length ? (
          <p className="text-[12.5px] text-fg-dim">A rebase only moves the plan&apos;s base.</p>
        ) : null}
        {v.error ? (
          <p role="alert" className="text-[12.5px] text-danger">
            {v.error}
          </p>
        ) : null}
        <p className="text-[12px] text-fg-mute">
          Rebase keeps your edits and takes the rest from layout.json. Start over replaces only the
          main line with the stops in layout.json now; proposals and stops you set aside stay.
          Either is one change you can undo in Changes.
        </p>
      </div>
    </Sheet>
  );
}
