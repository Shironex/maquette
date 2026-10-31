import type { ReactNode } from 'react';
import type { LibraryItem } from '@/ipc/client';
import type { ItemGroup } from '@/state/library';

export interface LibraryGroupsProps {
  groups: readonly ItemGroup[];
  list: boolean;
  /** One model's card or row. */
  render: (item: LibraryItem) => ReactNode;
}

const count = (n: number) => `${n} ${n === 1 ? 'model' : 'models'}`;

/** The models under a header each, "Tripo · 150 models", in the grid or the list. */
export default function LibraryGroups({ groups, list, render }: LibraryGroupsProps) {
  const sections = groups.map(g => {
    const rows = g.items.map(render);
    return (
      <section key={g.key} aria-label={g.label} className="flex flex-col gap-2.5">
        <h3 className="flex items-baseline gap-2 border-b border-line pb-1.5">
          <span className="text-[13px] font-semibold">{g.label}</span>
          <span className="text-meta text-fg-mute">{count(g.items.length)}</span>
        </h3>
        <ul
          className={
            list
              ? 'flex list-none flex-col gap-1'
              : 'grid list-none grid-cols-[repeat(auto-fill,minmax(172px,1fr))] gap-3'
          }
        >
          {rows}
        </ul>
      </section>
    );
  });
  return <div className="flex flex-col gap-5">{sections}</div>;
}
