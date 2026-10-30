import { Button } from '@maquette/ui';
import type { DiscardedModel } from '@/ipc/client';
import { formatAgo, formatBytes } from '@/lib/format';
import { useDiscardedList } from './DiscardedList.hooks';

export interface DiscardedListProps {
  /** How many to list before "and N more". */
  limit?: number;
}

/** Why a discarded model left, in a few words. */
function why(model: DiscardedModel, now: number): string {
  const parts = [
    model.replacedBy != null ? `replaced by v${model.replacedBy}` : null,
    model.bytes != null ? formatBytes(model.bytes) : null,
    model.discardedMs != null ? formatAgo(model.discardedMs, now) : null,
  ];
  return parts.filter(Boolean).join(' · ');
}

/**
 * The models discarded from the inbox, newest first, each with Restore. The
 * files wait in inbox/.discarded until someone removes them.
 */
export default function DiscardedList({ limit = 5 }: DiscardedListProps) {
  const { now, total, shown, more, restoring, restore } = useDiscardedList(limit);
  if (total === 0) return null;
  const rows = shown.map(d => (
    <li
      key={d.name}
      className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2"
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate font-semibold">{d.assetName ?? d.id}</span>
        <span className="block text-meta break-all text-fg-mute">
          {d.assetName ? `${d.id} · ` : ''}
          {why(d, now)}
        </span>
      </span>
      <Button
        size="sm"
        icon="arrow-counter-clockwise"
        loading={restoring === d.name}
        disabled={Boolean(restoring)}
        tooltip="Puts it back in the Inbox with its checks"
        aria-label={`Restore ${d.assetName ?? d.id}`}
        onClick={() => void restore(d.name)}
      >
        Restore
      </Button>
    </li>
  ));
  return (
    <section aria-labelledby="discarded-title" className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between gap-2">
        <h3
          id="discarded-title"
          className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
        >
          Recently discarded
        </h3>
        <span className="text-meta text-fg-mute">{total} kept</span>
      </div>
      <ul className="flex list-none flex-col gap-1.5">{rows}</ul>
      <p className="text-meta text-fg-mute">
        {more > 0 ? `And ${more} more. ` : ''}
        Files wait in inbox/.discarded in the workspace. Restore puts one back aimed at the asset it
        was going to.
      </p>
    </section>
  );
}
