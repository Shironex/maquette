import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { nextSort, type BoardPrefs, type SortKey } from '@/state/board-list';
import { useBoardPrefsStore } from '@/state/use-board-prefs-store';
import type { BoardListProps } from './BoardList.types';

const rowsOf = (root: HTMLElement | null) =>
  Array.from(root?.querySelectorAll<HTMLElement>('[data-row-id]') ?? []);

/**
 * The list's sort, filter and search (remembered per project), the row that
 * holds the tab stop, and the keys: arrows move, Shift extends the
 * selection, Space toggles it, Enter opens the row's menu.
 */
export function useBoardList({ rows, board, prefs }: BoardListProps) {
  const update = useBoardPrefsStore(s => s.update);
  const { project, selection, actions } = board;
  const [focused, setFocused] = useState<string | null>(null);
  const [menuUp, setMenuUp] = useState(false);
  const tableRef = useRef<HTMLDivElement>(null);
  const ids = rows.map(r => r.asset.id);
  const active = focused && ids.includes(focused) ? focused : (ids[0] ?? null);

  const change = (next: Partial<BoardPrefs>) => {
    if (project) update(project.id, next);
  };
  const sortBy = (key: SortKey) => change({ sort: nextSort(prefs.sort, key) });
  const clearFilters = () => change({ stage: 'all', query: '' });

  const focusRow = (id: string) => {
    setFocused(id);
    rowsOf(tableRef.current)
      .find(el => el.dataset.rowId === id)
      ?.focus();
  };

  // Closing a row's menu hands the focus back to its row.
  const lastMenu = useRef(actions.menuFor);
  useEffect(() => {
    const was = lastMenu.current;
    lastMenu.current = actions.menuFor;
    if (was && !actions.menuFor && document.activeElement === document.body) {
      rowsOf(tableRef.current)
        .find(el => el.dataset.rowId === was)
        ?.focus();
    }
  }, [actions.menuFor]);

  /** Opens (or closes) a row's menu, above the row when it sits low in the list. */
  const toggleMenu = (id: string) => {
    actions.setCopyFor(null);
    const box = tableRef.current?.getBoundingClientRect();
    const row = rowsOf(tableRef.current)
      .find(el => el.dataset.rowId === id)
      ?.getBoundingClientRect();
    setMenuUp(Boolean(box && row && row.top - box.top > box.height / 2));
    actions.setMenuFor(m => (m === id ? null : id));
  };

  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    const row = e.target as HTMLElement;
    const id = row.dataset.rowId;
    if (!id) return;
    const at = ids.indexOf(id);
    const step = { ArrowDown: 1, ArrowUp: -1 }[e.key];
    const jump = { Home: ids[0], End: ids[ids.length - 1] }[e.key];
    if (step !== undefined || jump !== undefined) {
      e.preventDefault();
      const next = jump ?? ids[at + (step ?? 0)];
      if (!next) return;
      if (e.shiftKey) {
        if (!selection.present.includes(id)) selection.check(id, true, false);
        selection.check(next, true, false);
      }
      focusRow(next);
    } else if (e.key === ' ') {
      e.preventDefault();
      selection.check(id, !selection.present.includes(id), false);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      toggleMenu(id);
    }
  };

  return {
    tableRef,
    /** True while the selection bar floats over the list's foot. */
    selecting: selection.present.length > 0,
    active,
    setFocused,
    menuUp,
    toggleMenu,
    onKeyDown,
    sortBy,
    clearFilters,
    change,
  };
}
