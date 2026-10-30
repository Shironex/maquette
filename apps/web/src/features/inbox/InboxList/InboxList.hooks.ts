import { useRef, useState } from 'react';
import { useWindowedList } from '@maquette/ui';
import { filterItems } from '@/state/triage';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';

/** An inbox row's height before it is measured. */
const ROW = 62;

/** The list column: the filter, the rows it keeps (windowed once long) and the row menus. */
export function useInboxList(shownId: string | null) {
  const { snapshot, select } = useInboxStore();
  const { filter, setFilter } = useTriageStore();
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const all = snapshot?.items ?? [];
  const items = filterItems(all, filter);
  const scrollRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const shown = useWindowedList({
    count: items.length,
    scrollRef,
    listRef,
    estimate: ROW,
    gap: 4,
  });
  return {
    snapshot,
    all,
    items,
    shownId,
    select,
    filter,
    setFilter,
    menuFor,
    toggleMenu: (id: string) => setMenuFor(m => (m === id ? null : id)),
    closeMenu: () => setMenuFor(null),
    scrollRef,
    listRef,
    shown,
  };
}
