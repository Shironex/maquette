import { Button, cn, IconButton } from '@maquette/ui';
import { useOverlapsPanel, type OverlapView } from './OverlapsPanel.hooks';

/**
 * Beside a Game mechanics canvas: the system cards that overlap (a shared
 * verb, look or key reads as one system to a player), each with Show and
 * Mark as fine, and the pairs already marked fine below.
 */
export default function OverlapsPanel() {
  const v = useOverlapsPanel();
  const card = (o: OverlapView) => (
    <div
      key={o.key}
      role="group"
      aria-label={o.title}
      className={cn('plan-overlap', o.fine && 'is-fine')}
    >
      <h3 className="text-[13px] leading-[17px] font-semibold break-words text-fg">{o.title}</h3>
      <p className="mt-1 text-[12px] leading-4 text-fg-dim">{o.detail}</p>
      <div className="mt-2.5 flex justify-end gap-1.5">
        <Button size="sm" variant="ghost" aria-label={`Show: ${o.title}`} onClick={() => v.show(o)}>
          Show
        </Button>
        {o.fine ? (
          <span className="self-center text-[11.5px] text-fg-mute">Marked as fine</span>
        ) : (
          <Button
            size="sm"
            disabled={v.busy}
            disabledReason="A change is on its way"
            aria-label={`Mark as fine: ${o.title}`}
            tooltip="Keeps this pair as it is; the flag comes back when either card's words change"
            onClick={() => v.markFine(o)}
          >
            Mark as fine
          </Button>
        )}
      </div>
    </div>
  );
  const open = v.open.map(card);
  const fine = v.fine.map(card);
  return (
    <aside aria-label="Overlaps" className="plan-side">
      <div className="plan-side-head">
        <h2 className="flex-1 text-[15px] leading-5 font-semibold text-fg">
          Overlaps
          <span className="ml-2 text-[12px] font-normal text-fg-mute">{v.count}</span>
        </h2>
        <IconButton icon="x" size="sm" variant="ghost" label="Close Overlaps" onClick={v.close} />
      </div>
      <div className="plan-side-body">
        {open.length ? (
          open
        ) : (
          <p className="text-meta text-fg-mute">
            No overlaps. Each system has its own verb, look and key.
          </p>
        )}
        {fine.length ? (
          <section aria-label="Marked as fine" className="flex flex-col gap-2.5">
            <h3 className="mt-1 text-[11.5px] font-medium tracking-[0.05em] text-fg-mute uppercase">
              Marked as fine
            </h3>
            {fine}
          </section>
        ) : null}
      </div>
      <p className="plan-side-foot">
        I flag two systems that share a verb, a look or a key. Same main word counts as the same.
      </p>
    </aside>
  );
}
