import { cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { GROUPS, groupCounts, type TriageFilter } from '@/state/triage';

export interface TriageChipsProps {
  items: readonly InboxItem[];
  value: TriageFilter;
  onChange: (filter: TriageFilter) => void;
}

/** Short names for the chips, which sit in a narrow column. */
const SHORT: Record<TriageFilter, string> = {
  all: 'All',
  pass: 'Pass',
  review: 'To review',
  replaced: 'Replaced',
  unreadable: 'Unreadable',
};

/**
 * The filter over the inbox list: every model, or the ones whose checks
 * pass, need a look, a redo replaced, or could not be read. A group with no
 * models has no chip, unless it is the one picked.
 */
export default function TriageChips({ items, value, onChange }: TriageChipsProps) {
  const counts = groupCounts(items);
  const chips: TriageFilter[] = [
    'all',
    ...GROUPS.map(g => g.id).filter(id => counts[id] > 0 || id === value),
  ];
  const buttons = chips.map(id => (
    <button
      key={id}
      type="button"
      aria-pressed={value === id}
      onClick={() => onChange(id)}
      className={cn(
        'inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-full border border-line px-2.5 text-[12px] leading-none font-medium text-fg-dim hover:bg-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus',
        value === id && 'border-clay-line bg-clay-soft text-fg hover:bg-clay-soft'
      )}
    >
      {SHORT[id]} <span className="font-mono text-[11px] text-fg-mute">{counts[id]}</span>
    </button>
  ));
  return (
    <div role="group" aria-label="Show" className="flex flex-wrap gap-1.5">
      {buttons}
    </div>
  );
}
