import { Button, StatusChip, type Status } from '@maquette/ui';
import type { PackRecord, PackState } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { formatAgo, formatBytes } from '@/lib/format';
import { settled } from '@/state/packs';

const CHIP: Record<PackState, { status: Status; label: string }> = {
  running: { status: 'running', label: 'Running' },
  waiting: { status: 'review', label: 'Needs you' },
  zipping: { status: 'running', label: 'Zipping' },
  done: { status: 'done', label: 'Done' },
  stopped: { status: 'queued', label: 'Stopped' },
  failed: { status: 'failed', label: 'Failed' },
};

export interface RecentPacksProps {
  recent: readonly PackRecord[];
  /** The pack on show, left out of the list. */
  current: string | null;
  busy: boolean;
  onOpen: (id: string) => void;
  onResume: (id: string) => void;
}

/** The newest packs: open one to see its checks, or go on with one that stopped. */
export default function RecentPacks({ recent, current, busy, onOpen, onResume }: RecentPacksProps) {
  const shown = recent.filter(r => r.id !== current);
  if (!shown.length) return null;
  const rows = shown.map(r => {
    const chip = CHIP[r.state];
    const at = r.items.filter(settled).length + 1;
    const detail =
      r.state === 'stopped' || r.state === 'failed'
        ? (r.message ?? `Stopped at model ${at} of ${r.items.length}`)
        : [
            `${r.items.length} models`,
            r.bytes ? formatBytes(r.bytes) : null,
            formatAgo(Date.parse(r.createdAt), clock.nowMs()),
          ]
            .filter(Boolean)
            .join(' · ');
    return (
      <li key={r.id} className="flex items-center gap-3 border-t border-line py-2">
        <div className="flex min-w-0 flex-1 flex-col">
          <b className="truncate font-mono text-[12.5px]">{r.name}</b>
          <span className="truncate text-meta text-fg-mute">{detail}</span>
        </div>
        <StatusChip status={chip.status} label={chip.label} />
        {r.state === 'stopped' ? (
          <Button size="sm" variant="primary" disabled={busy} onClick={() => onResume(r.id)}>
            Go on from {at}
          </Button>
        ) : (
          <Button size="sm" onClick={() => onOpen(r.id)}>
            Open
          </Button>
        )}
      </li>
    );
  });
  return <ul className="flex list-none flex-col">{rows}</ul>;
}
