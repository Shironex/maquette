import { formatK } from '@/lib/format';
import type { LibraryStats as Stats } from '@/state/library';
import { StatTile } from '@/features/common/StatTile';

export interface LibraryStatsProps {
  stats: Stats;
}

/** How many models, what reuse saved, and how many copies have an update waiting. */
export default function LibraryStats({ stats }: LibraryStatsProps) {
  return (
    <div className="flex gap-3">
      <StatTile label="Models" value={String(stats.models)} note={`${stats.shared} shared`} />
      <StatTile
        label="Reuse saved"
        value={formatK(stats.saved)}
        note="credits"
        tone={stats.saved > 0 ? 'text-sage' : undefined}
      />
      <StatTile
        label="Updates"
        value={String(stats.updates)}
        note={stats.updates === 1 ? 'copy behind' : 'copies behind'}
        tone={stats.updates > 0 ? 'text-clay-text' : undefined}
      />
    </div>
  );
}
