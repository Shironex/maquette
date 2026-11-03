import { Button, Icon } from '@maquette/ui';
import { headOf, useRemovalAsks } from './RemovalAsks.hooks';

/**
 * At the top of the Changes panel: each agent's ask to remove nodes it may
 * not remove itself, the nodes by name and the reason it gave. Only the
 * person answers. Remove takes the nodes off as the person's own change,
 * which they can undo; Keep it leaves the canvas as it is.
 */
export default function RemovalAsks() {
  const v = useRemovalAsks();
  if (!v.proposals.length) return null;
  const asks = v.proposals.map(p => {
    const names = p.nodes.map(n => (
      <li key={n.id} className="break-words">
        {n.name}
        {v.noteOf(n.state) ? <span className="text-clay-text"> ({v.noteOf(n.state)})</span> : null}
      </li>
    ));
    return (
      <div key={p.id} role="group" aria-label={headOf(p)} className="plan-ask">
        <h3 className="flex items-center gap-1.5 text-[13px] leading-[17px] font-semibold text-fg">
          <Icon name="question" size={14} className="flex-none text-[var(--plan-agent)]" />
          {headOf(p)}
        </h3>
        <ul className="mt-1.5 flex flex-col gap-0.5 text-[12.5px] leading-[17px] text-fg-dim">
          {names}
        </ul>
        {p.reason ? (
          <p className="mt-1.5 text-[12px] leading-4 break-words text-fg-mute">
            It says {p.reason.replace(/[.\s]+$/, '')}.
          </p>
        ) : null}
        {p.stale ? (
          <p className="mt-1.5 text-[12px] leading-4 text-clay-text">
            {p.nodes.length === 1 ? 'The node' : 'A node'} changed or went since it asked, so it can
            only be kept.
          </p>
        ) : null}
        <div className="mt-2.5 flex justify-end gap-1.5">
          <Button size="sm" disabled={v.busy} onClick={() => v.keep(p.id)}>
            Keep it
          </Button>
          <Button
            size="sm"
            variant="danger"
            disabled={v.busy || p.stale}
            disabledReason={
              p.stale ? 'A node changed or went since the agent asked' : 'An answer is on its way'
            }
            aria-label={`Remove: ${v.namesOf(p)}`}
            onClick={() => v.remove(p.id)}
          >
            Remove
          </Button>
        </div>
      </div>
    );
  });
  return (
    <section aria-label="Waiting for you" className="flex flex-col gap-2.5">
      {asks}
    </section>
  );
}
