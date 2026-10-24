import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

export interface KeyValueItem {
  label: string;
  value: ReactNode;
  mono?: boolean;
}

export interface KeyValueListProps {
  items: readonly KeyValueItem[];
  columns?: number;
  className?: string;
}

export function KeyValueList({ items, columns = 2, className }: KeyValueListProps) {
  return (
    <dl
      className={cn('m-0 grid gap-x-4 gap-y-3', className)}
      style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
    >
      {items.map(it => (
        <div key={it.label} className="flex min-w-0 flex-col gap-0.5">
          <dt className="text-meta text-fg-mute">{it.label}</dt>
          <dd
            title={typeof it.value === 'string' ? it.value : undefined}
            className={cn(
              'm-0 truncate font-medium text-fg tabular-nums',
              it.mono && 'font-mono text-[12px] font-normal'
            )}
          >
            {it.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
