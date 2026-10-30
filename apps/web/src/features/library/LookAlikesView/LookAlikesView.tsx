import { Button, EmptyState } from '@maquette/ui';
import type { LibraryItem, LookAlikes } from '@/ipc/client';
import { formatClock } from '@/lib/format';
import { pairKey } from '@/state/use-look-alikes-store';
import { LookAlikeCard } from '../LookAlikeCard';
import LookAlikeSide from './LookAlikeSide';
import { useLookAlikesView } from './LookAlikesView.hooks';

export interface LookAlikesViewProps {
  report: LookAlikes | null;
  items: readonly LibraryItem[];
}

/** How many pairs show whole before the rest wait folded. */
const OPEN = 2;

/**
 * Look-alikes: models that look the same under two names and different
 * models under one name, each pair with its choices, the merges that can
 * still be undone, and how the match works on the side.
 */
export default function LookAlikesView({ report, items }: LookAlikesViewProps) {
  const { undo, scanning, error, isOpened, open } = useLookAlikesView();
  const byId = new Map(items.map(i => [i.entry.id, i]));
  if (!report) {
    return (
      <section
        aria-label="Look-alikes"
        aria-busy={scanning}
        className="flex flex-1 items-center justify-center p-6"
      >
        {error ? (
          <p role="alert" className="text-danger">
            {error}
          </p>
        ) : (
          <p className="text-fg-mute">Comparing the pictures</p>
        )}
      </section>
    );
  }
  const pairs = report.pairs.filter(p => byId.has(p.a) && byId.has(p.b));
  const cards = pairs.map((p, n) => {
    const key = pairKey(p);
    return (
      <LookAlikeCard
        key={key}
        pair={p}
        a={byId.get(p.a)!}
        b={byId.get(p.b)!}
        open={n < OPEN || isOpened(key)}
        onOpen={() => open(key)}
      />
    );
  });
  const merges = report.merges.map(m => {
    const became = m.versions.map(v => `v${v}`).join(' and ');
    return (
      <li
        key={m.merged}
        className="flex items-center gap-3 rounded-card border border-line px-3.5 py-2.5"
      >
        <span className="rounded-pill bg-sage-soft px-2 text-[11px] leading-5 font-medium text-sage">
          merged
        </span>
        <b className="truncate">{m.intoName}</b>
        <span className="min-w-0 truncate text-meta text-fg-mute">
          {m.mergedName} became {became} · {formatClock(Date.parse(m.at))}
        </span>
        <span className="flex-1" />
        <Button
          size="sm"
          icon="arrow-u-up-left"
          tooltip="Puts both models back as they were"
          onClick={() => void undo(m.merged)}
        >
          Undo
        </Button>
      </li>
    );
  });
  const unseen = report.unseen.map(id => byId.get(id)?.entry.name ?? id);
  return (
    <>
      <section
        aria-label="Look-alikes"
        className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
      >
        <div>
          <h2 className="text-panel-title">Look-alikes</h2>
          <p className="text-meta text-fg-mute">
            {pairs.length} open in {items.length} models. Found by picture, the same way the Inbox
            matches downloads, and by name.
          </p>
        </div>
        {cards.length ? (
          <ul className="flex list-none flex-col gap-3">{cards}</ul>
        ) : (
          <EmptyState
            icon="check-circle"
            title="No look-alikes"
            body="No two models look the same or share a name. A new scan runs when you open this again."
          />
        )}
        {merges.length ? <ul className="flex list-none flex-col gap-2">{merges}</ul> : null}
      </section>
      <LookAlikeSide report={report} models={items.length} unseen={unseen} />
    </>
  );
}
