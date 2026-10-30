import { Button, TextField, cn } from '@maquette/ui';
import type { LibraryItem, LookAlikePair } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { latestOf } from '@/state/library';
import { LibraryPicture } from '../LibraryPicture';
import { useLookAlikeCard } from './LookAlikeCard.hooks';

export interface LookAlikeCardProps {
  pair: LookAlikePair;
  a: LibraryItem;
  b: LibraryItem;
  /** Shown whole; folded it is one line with Review. */
  open: boolean;
  onOpen: () => void;
}

const score = (n: number | null) => (n === null ? 'no picture' : n.toFixed(2));

/** "v1 · Hollow Vale", "v2 · added 3 days ago". */
function where(item: LibraryItem): string {
  const v = latestOf(item);
  const use = item.uses[0]?.projectName;
  const when = `added ${formatAgo(Date.parse(item.entry.createdAt), clock.nowMs())}`;
  return `v${v?.version ?? 1} · ${use ?? when}`;
}

function Side({ item }: { item: LibraryItem }) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1.5">
      <LibraryPicture
        item={item}
        className="aspect-[4/3] w-full rounded-control border border-line"
      />
      <b className="truncate">{item.entry.name}</b>
      <span className="truncate font-mono text-[11.5px] text-fg-mute">{where(item)}</span>
    </div>
  );
}

function Meter({ label, value }: { label: string; value: number | null }) {
  return (
    <div className="flex items-center gap-2 text-meta">
      <span className="w-16 text-fg-mute">{label}</span>
      <span className="h-1.5 flex-1 overflow-hidden rounded-pill bg-raised">
        <span
          className="block h-full bg-clay"
          style={{ width: `${Math.round((value ?? 0) * 100)}%` }}
        />
      </span>
      <span className="w-9 text-right font-mono text-fg-dim">{score(value)}</span>
    </div>
  );
}

/**
 * One pair to decide: two models that look the same (merge as versions,
 * keep both, not alike), or two models under one name (rename both, keep
 * as is). Folded, it waits on one line for a Review.
 */
export default function LookAlikeCard({ pair, a, b, open, onOpen }: LookAlikeCardProps) {
  const card = useLookAlikeCard(pair, a, b);
  const { working, first, second } = card;
  const same = pair.kind === 'same-look';
  const title = `${a.entry.name} and ${b.entry.name}`;
  const tag = (
    <span
      className={cn(
        'rounded-pill px-2 text-[11px] leading-5 font-medium',
        same ? 'bg-clay-soft text-clay-text' : 'bg-amber-soft text-amber'
      )}
    >
      {same ? 'same look' : 'same name'}
    </span>
  );
  const looks = `looks like ${score(pair.score)}`;

  if (!open) {
    return (
      <li className="flex items-center gap-3 rounded-card border border-line bg-raised px-3.5 py-2.5">
        {tag}
        <b className="min-w-0 truncate">{title}</b>
        <span className="text-meta text-fg-mute">{looks}</span>
        <span className="flex-1" />
        <Button size="sm" onClick={onOpen} tooltip="Opens the pair to decide">
          Review
        </Button>
      </li>
    );
  }
  return (
    <li
      aria-label={title}
      className="flex flex-col gap-3 rounded-card border border-line bg-raised p-4"
    >
      <div className="flex items-center gap-2.5">
        {tag}
        <b className="min-w-0 truncate">{title}</b>
        <span className="flex-1" />
        <span className="font-mono text-meta text-fg-mute">{looks}</span>
      </div>
      <div className="flex gap-3">
        <Side item={a} />
        <Side item={b} />
        {same ? (
          <div className="flex w-[220px] flex-none flex-col justify-center gap-2">
            <Meter label="Outline" value={pair.shape} />
            <Meter label="Colour" value={pair.colour} />
            <Meter label="Box ratio" value={pair.ratio} />
          </div>
        ) : null}
      </div>
      {same ? (
        <>
          <p className="text-meta text-fg-dim">
            Merged, {b.entry.name} becomes the next version of {a.entry.name}; a project that uses
            it keeps the model it has until you update it.
          </p>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="primary"
              icon="stack"
              loading={working}
              disabled={working}
              tooltip="Makes the second model a newer version of the first; Undo puts it back"
              onClick={card.merge}
            >
              Merge as versions
            </Button>
            <Button disabled={working} onClick={card.keepBoth}>
              Keep both
            </Button>
            <Button variant="ghost" disabled={working} onClick={card.notAlike}>
              Not alike
            </Button>
          </div>
        </>
      ) : (
        <>
          <p className="text-meta text-fg-dim">
            Two different models share a name. Give each a name that says what it is.
          </p>
          <div className="flex flex-wrap items-end gap-2">
            <TextField
              label="First"
              value={first}
              maxLength={80}
              onChange={e => card.setFirst(e.target.value)}
              fieldClassName="w-[200px]"
            />
            <TextField
              label="Second"
              value={second}
              maxLength={80}
              onChange={e => card.setSecond(e.target.value)}
              fieldClassName="w-[200px]"
            />
            <Button
              variant="primary"
              loading={working}
              disabled={working || card.clash}
              disabledReason="Give the two different names"
              onClick={card.renameBoth}
            >
              Rename both
            </Button>
            <Button variant="ghost" disabled={working} onClick={card.keepNames}>
              Keep as is
            </Button>
          </div>
        </>
      )}
    </li>
  );
}
