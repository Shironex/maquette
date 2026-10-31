import { Button } from '@maquette/ui';
import type { PackRecord } from '@/ipc/client';
import { useEarlierPacks } from './EarlierPacks.hooks';
import RecentPacks from './RecentPacks';

export interface EarlierPacksProps {
  recent: readonly PackRecord[];
  busy: boolean;
  onOpen: (id: string) => void;
  onResume: (id: string) => void;
}

/** The way back to a pack that ran before, above the form for a new one. */
export default function EarlierPacks({ recent, busy, onOpen, onResume }: EarlierPacksProps) {
  const { all, shown, more, toggle } = useEarlierPacks(recent);
  if (!recent.length) return null;
  return (
    <section aria-label="Earlier packs" className="flex flex-col gap-1">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
          Earlier packs
        </h3>
        {more ? (
          <Button size="sm" variant="ghost" onClick={toggle}>
            {all ? 'Show fewer' : `Show all ${recent.length}`}
          </Button>
        ) : null}
      </div>
      <RecentPacks recent={shown} current={null} busy={busy} onOpen={onOpen} onResume={onResume} />
    </section>
  );
}
