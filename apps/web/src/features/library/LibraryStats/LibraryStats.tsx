import { formatK } from '@/lib/format';
import type { LibraryStats as Stats } from '@/state/library';

export interface LibraryStatsProps {
  stats: Stats;
}

function Stat({
  label,
  value,
  note,
  tone,
}: {
  label: string;
  value: string;
  note: string;
  tone?: string;
}) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1 rounded-card border border-line bg-raised px-4 py-3">
      <span className="text-meta text-fg-mute">{label}</span>
      <span className={tone ?? 'text-fg'}>
        <span className="text-[22px] leading-7 font-semibold tracking-[-0.01em]">{value}</span>{' '}
        <small className="text-meta text-fg-mute">{note}</small>
      </span>
    </div>
  );
}

/** How many models, what reuse saved, and how many copies have an update waiting. */
export default function LibraryStats({ stats }: LibraryStatsProps) {
  return (
    <div className="flex gap-3">
      <Stat label="Models" value={String(stats.models)} note={`${stats.shared} shared`} />
      <Stat
        label="Credits saved by reuse"
        value={formatK(stats.saved)}
        note="across projects"
        tone={stats.saved > 0 ? 'text-sage' : undefined}
      />
      <Stat
        label="Updates waiting"
        value={String(stats.updates)}
        note={stats.updates === 1 ? 'copy behind' : 'copies behind'}
        tone={stats.updates > 0 ? 'text-clay-text' : undefined}
      />
    </div>
  );
}
