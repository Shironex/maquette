/**
 * The side column of a library view: options under small headings, each
 * with its count, pressed while it filters. The Models and Animations
 * views both use it.
 */
import type { ReactNode } from 'react';
import { cn } from '@maquette/ui';

export function FilterOption({
  label,
  count,
  on,
  onClick,
}: {
  label: ReactNode;
  /** Left out while the count is not known yet. */
  count?: number;
  on: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onClick}
      className={cn(
        'flex h-8 w-full cursor-pointer items-center gap-2 rounded-control px-2.5 text-left text-fg-dim hover:bg-hover hover:text-fg',
        on && 'bg-clay-soft font-medium text-fg hover:bg-clay-soft'
      )}
    >
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {count === undefined ? null : (
        <span className="font-mono text-[11px] text-fg-mute">{count}</span>
      )}
    </button>
  );
}

export function FilterGroup({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div role="group" aria-label={title} className="flex flex-col gap-0.5">
      <div className="px-2.5 pt-1 pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
        {title}
      </div>
      {children}
    </div>
  );
}

export default function FilterList({ children }: { children: ReactNode }) {
  return (
    <section
      aria-label="Filters"
      className="flex w-[232px] flex-none flex-col gap-5 overflow-y-auto border-r border-line bg-panel px-3 py-[18px]"
    >
      {children}
    </section>
  );
}
