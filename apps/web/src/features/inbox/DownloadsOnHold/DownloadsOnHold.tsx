import { Button } from '@maquette/ui';
import type { Arrival } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useDownloadsOnHold } from './DownloadsOnHold.hooks';

/** What a held download has to go on until it gets its picture. */
function waitsFor(arrival: Arrival): string {
  const best = arrival.suggestions?.[0];
  const close =
    best && (best.name ?? 0) >= 0.4 ? `Name is close to ${best.assetName}.` : 'No name match.';
  return `Waits for a picture. ${close}`;
}

/** Downloads waiting for Blender to come back, which can still be matched by hand. */
export default function DownloadsOnHold() {
  const now = useNow(30_000);
  const { held, openPicker } = useDownloadsOnHold();
  if (held.length === 0) return null;
  const rows = held.map(a => (
    <li
      key={a.id}
      className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2"
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate font-mono text-[11.5px] text-fg">{a.name}</span>
        <span className="block truncate text-meta text-fg-mute">{waitsFor(a)}</span>
      </span>
      {a.atMs != null ? (
        <span className="flex-none text-meta text-fg-mute">{formatAgo(a.atMs, now)}</span>
      ) : null}
    </li>
  ));
  return (
    <section aria-labelledby="on-hold-title" className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <h3
          id="on-hold-title"
          className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
        >
          Downloads on hold · {held.length}
        </h3>
        <Button size="sm" variant="secondary" onClick={openPicker}>
          Match these by hand
        </Button>
      </div>
      <ul className="flex list-none flex-col gap-1.5">{rows}</ul>
    </section>
  );
}
